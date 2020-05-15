extern crate freetype;
extern crate harfbuzz_rs;
extern crate math_render;

use self::harfbuzz_rs::{Face, Font};
use math_render::shaper::HarfbuzzShaper;

pub fn get_bytes() -> &'static [u8] {
    include_bytes!("testfiles/latinmodern-math.otf")
}

thread_local! {
    pub static TEST_FONT: HarfbuzzShaper<'static> = {
        let face = Face::new(get_bytes(), 0);
        let font = Font::new(face);
        HarfbuzzShaper::new(font.into())
    };
}
