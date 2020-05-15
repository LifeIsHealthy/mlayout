#![allow(dead_code)]

extern crate math_render;
extern crate freetype;

extern crate image;

use std::path::Path;
use std::ops::{Add, Sub, Mul, MulAssign, Div, DivAssign};

use freetype::Library;
use freetype::render_mode::RenderMode;
use freetype::Face;
use freetype::face::LoadFlag;
use freetype::ffi::{FT_Library_SetLcdFilter, FT_LCD_FILTER_DEFAULT, FT_LOAD_TARGET_LCD};

#[derive(Copy, Clone)]
struct Color3f {
    r: f32,
    g: f32,
    b: f32,
}
impl Add<Color3f> for Color3f {
    type Output = Color3f;
    fn add(self, _rhs: Color3f) -> Color3f {
        Color3f {
            r: self.r + _rhs.r,
            g: self.g + _rhs.g,
            b: self.b + _rhs.b,
        }
    }
}
impl Add<f32> for Color3f {
    type Output = Color3f;
    fn add(self, _rhs: f32) -> Color3f {
        Color3f {
            r: self.r + _rhs,
            g: self.g + _rhs,
            b: self.b + _rhs,
        }
    }
}
impl Mul<Color3f> for Color3f {
    type Output = Color3f;
    fn mul(self, _rhs: Color3f) -> Color3f {
        Color3f {
            r: self.r * _rhs.r,
            g: self.g * _rhs.g,
            b: self.b * _rhs.b,
        }
    }
}
impl Mul<f32> for Color3f {
    type Output = Color3f;
    fn mul(self, _rhs: f32) -> Color3f {
        Color3f {
            r: self.r * _rhs,
            g: self.g * _rhs,
            b: self.b * _rhs,
        }
    }
}
impl MulAssign<f32> for Color3f {
    fn mul_assign(&mut self, _rhs: f32) {
        self.r *= _rhs;
        self.g *= _rhs;
        self.b *= _rhs;
    }
}

impl Div<f32> for Color3f {
    type Output = Color3f;
    fn div(self, _rhs: f32) -> Color3f {
        Color3f {
            r: self.r / _rhs,
            g: self.g / _rhs,
            b: self.b / _rhs,
        }
    }
}

impl DivAssign<f32> for Color3f {
    fn div_assign(&mut self, _rhs: f32) {
        self.r /= _rhs;
        self.g /= _rhs;
        self.b /= _rhs;
    }
}
impl Sub<Color3f> for f32 {
    type Output = Color3f;

    fn sub(self, _rhs: Color3f) -> Color3f {
        Color3f {
            r: self - _rhs.r,
            g: self - _rhs.g,
            b: self - _rhs.b,
        }
    }
}


struct Color4 {
    r: u8,
    g: u8,
    b: u8,
    alpha: u8,
}
impl Color4 {
    fn from_color3f(mut color3: Color3f, alpha: f32) -> Color4 {
        color3 *= 255f32;
        Color4 {
            r: color3.r as u8,
            g: color3.g as u8,
            b: color3.b as u8,
            alpha: (alpha * 255f32) as u8,
        }
    }
}
impl IntoIterator for Color4 {
    type Item = u8;
    type IntoIter = ColorIter<Color4>;
    fn into_iter(self) -> ColorIter<Color4> {
        ColorIter {
            color: self,
            state: 0,
        }
    }
}
struct ColorIter<C> {
    color: C,
    state: u8,
}
impl Iterator for ColorIter<Color4> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        let result = match self.state {
            0 => self.color.r,
            1 => self.color.g,
            2 => self.color.b,
            3 => self.color.alpha,
            _ => return None,
        };
        self.state += 1;
        Some(result)
    }
}

struct Renderer {
    lib: Library,
    ft_face: Face<'static>,
}
impl Renderer {
    fn new() -> Renderer {
        // Init the library
        let lib = Library::init().unwrap();
        unsafe {
            FT_Library_SetLcdFilter(lib.raw(), FT_LCD_FILTER_DEFAULT);
        }
        let face = lib.new_face("/Library/Fonts/latinmodern-math.otf", 0).unwrap();
        face.set_char_size(400 * 64, 0, 50, 0).unwrap();

        //hb::hb_o

        Renderer {
            lib: lib,
            ft_face: face,
        }
    }

    fn render_glyph(&self, glyph: u32) -> (Vec<u8>, i32, i32) {
        self.ft_face.load_glyph(glyph, LoadFlag::from_bits_truncate(FT_LOAD_TARGET_LCD)).unwrap();
        //self.ft_face.load_glyph(glyph, LoadFlag::empty()).unwrap();

        let glyph = self.ft_face.glyph();
        glyph.render_glyph(RenderMode::Lcd).unwrap();
        //glyph.render_glyph(RenderMode::Normal);
        let bitmap = glyph.bitmap();
        println!("{:?}, pitch {:?}",
                 bitmap.pixel_mode().unwrap(),
                 bitmap.pitch() - bitmap.width());

        let mut pixel_num = 0u32;
        let pitch = bitmap.pitch() as u32;
        let width = bitmap.width() as u32;
        let height = bitmap.rows() as u32;
        let buffer = bitmap.buffer();
        let iterator = std::iter::repeat([100u8, 0u8, 0u8, 255u8])
            .take((width * height / 3) as usize)
            .flat_map(|t| {
                let mut index = pixel_num + (pixel_num / width) * (pitch - width);
                pixel_num += 1;
                let red1: f32 = (buffer[index as usize] as f32) / 255f32;
                index = pixel_num + (pixel_num / width) * (pitch - width);
                pixel_num += 1;
                let green1: f32 = (buffer[index as usize] as f32) / 255f32;
                index = pixel_num + (pixel_num / width) * (pitch - width);
                pixel_num += 1;
                let blue1: f32 = (buffer[index as usize] as f32) / 255f32;

                let new_alpha: f32 = red1.max(green1.max(blue1));

                let mut mask = Color3f {
                    r: red1,
                    g: green1,
                    b: blue1,
                };
                mask /= new_alpha;

                let mut src = Color3f {
                    r: t[0] as f32,
                    g: t[1] as f32,
                    b: t[2] as f32,
                };
                src /= 255f32;

                let blend = 1f32 - mask + mask * src;
                Color4::from_color3f(blend, new_alpha)
            });
        (iterator.collect(), bitmap.width(), bitmap.rows())
    }
}

fn main() {
    let renderer = Renderer::new();
    let (buffer, width, height) = renderer.render_glyph(22);
    println!("lenth {:?}, width {:?}, height {:?}",
             buffer.len(),
             width,
             height);
    image::save_buffer(&Path::new("image.png"),
                       &buffer,
                       (width / 3i32) as u32,
                       height as u32,
                       image::ColorType::Rgb8)
            .unwrap();
}
