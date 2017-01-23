extern crate math_render;
extern crate freetype;
extern crate harfbuzz_rs;

use math_render::shaper::HarfbuzzShaper;
use self::harfbuzz_rs::Face;


pub fn get_bytes() -> &'static [u8] {
    include_bytes!("testfiles/latinmodern-math.otf")
}

thread_local! {
    pub static TEST_FONT: HarfbuzzShaper<'static> = {
        let font = Face::new(get_bytes(), 0).create_font();
        HarfbuzzShaper::new(font)
    };
}
