extern crate math_render;
extern crate freetype;

use std::fmt::Debug;
use math_render::font::MathFont;


pub fn get_bytes() -> &'static [u8] {
    include_bytes!("testfiles/latinmodern-math.otf")
}

thread_local! {
    pub static FT_LIB: freetype::Library = freetype::Library::init().unwrap();
}

pub fn test_font() -> MathFont<'static> {
    FT_LIB.with(|ft_lib| {
        MathFont::from_bytes(get_bytes(), 0, ft_lib)
    })
}

#[allow(dead_code)]
pub fn layout_list<T: Debug>(list: math_render::MathExpression<T>) -> math_render::math_box::MathBox<T> {
    FT_LIB.with(|ft_lib| {
        math_render::layout(list, &test_font(), ft_lib)
    })
}
