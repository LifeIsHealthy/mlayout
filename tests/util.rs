extern crate math_render;
extern crate freetype;
extern crate harfbuzz_rs;

use std::fmt::Debug;
use math_render::font::HarfbuzzShaper;
use self::harfbuzz_rs::Face;


pub fn get_bytes() -> &'static [u8] {
    include_bytes!("testfiles/latinmodern-math.otf")
}

thread_local! {
    pub static FT_LIB: freetype::Library = freetype::Library::init().unwrap();
}

pub fn test_font() -> HarfbuzzShaper<'static> {
    FT_LIB.with(|ft_lib| {
        let font = Face::new(get_bytes(), 0).create_font();
        HarfbuzzShaper::new(font)
    })
}

#[allow(dead_code)]
pub fn layout_list<T: Debug>(list: math_render::MathExpression<T>) -> math_render::math_box::MathBox<T> {
    FT_LIB.with(|ft_lib| {
        math_render::layout(list, &test_font(), ft_lib)
    })
}
