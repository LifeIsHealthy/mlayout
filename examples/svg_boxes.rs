extern crate svg;
extern crate math_render;
extern crate freetype;
extern crate harfbuzz_rs;

use math_render::*;
use math_render::math_box::*;
use math_render::shaper::*;

use svg::Document;
use svg::node::element::{Rectangle, Line, Group, Path, Description};
use svg::node::element::path::Data;
use svg::node::{Node, Text};

use freetype::{face, Vector};
use freetype::face::Face as FT_Face;
use freetype::error::FtResult;
use freetype::outline::Curve;

use harfbuzz_rs::{Face, FontFuncsBuilder};

macro_rules! render_test {
    ( $y:expr, $( $x:expr ),+ ) => (
        $(
            let test = include_bytes!(concat!("../tests/testfiles/", $x,".xml"));
            render_mathml(test, $y, concat!("svg_images/", $x, ".svg"));
        )+
    );
}

fn main() {

    let font = include_bytes!("/Users/mr/Library/Fonts/latinmodern-math.otf");
//    let font = include_bytes!("/Library/Fonts/Microsoft/Cambria Math.ttf");
//    let font = include_bytes!("/Users/mr/Library/Fonts/Asana-Math-2.otf");
//    let font = include_bytes!("/Users/mr/Library/Fonts/texgyreschola-math.otf");
//    let font = include_bytes!("/Users/mr/Library/Fonts/xits-math.otf");
//    let font = include_bytes!("/Users/mr/Library/Fonts/STIX2Math.otf");

    render_test!(font,
                 "schrödinger",
                 "Vaccent",
                 "horizontal_glyphs",
                 "frac",
                 "pythagoras",
                 "italic_sup",
                 "math_kern",
                 "root",
                 "sum",
                 "euler",
                 "limit",
                 "stokes",
                 "parentheses",
                 "integrals",
                 "horizontal_stretch");
    // render_test!(font, "pythagoras");

    // let library = freetype::Library::init().unwrap();
    // let face = library.new_memory_face(&font[..], 0).unwrap();
    // let font = Face::new(&font[..], 0).create_font();
    // let shaper = HarfbuzzShaper::new(font);
    // let math_box = shaper.shape_stretchy("√", false, 6000, Default::default());
    //
    //
    // render_box(math_box, &shaper, &face, "svg_images/big_root.svg");
}

fn render_mathml(file: &[u8], font_bytes: &[u8], output_name: &str) {
    let list = mathmlparser::parse(&file[..]).expect("invalid parse");

    let mut font_funcs = FontFuncsBuilder::new();
    font_funcs.set_glyph_extents_func(|_, ft_face, glyph| {
        let result = FT_Face::load_glyph(ft_face, glyph, face::NO_SCALE);
        if result.is_err() {
            return None;
        }
        let metrics = ft_face.glyph().metrics();
        Some(GlyphExtents {
            width: metrics.width as i32,
            height: -metrics.height as i32,
            x_bearing: metrics.horiBearingX as i32,
            y_bearing: metrics.horiBearingY as i32,
        })
    });
    let font_funcs = font_funcs.finish();
    let library = freetype::Library::init().unwrap();
    let face = library.new_memory_face(font_bytes, 0).unwrap();
    let mut font = Face::new(font_bytes, 0).create_font().create_sub_font();
    font.set_font_funcs(&font_funcs, &face);
    let shaper = HarfbuzzShaper::new(font);

    let parsed_box = math_render::layout(list, &shaper);

    render_box(parsed_box, &shaper, &face, output_name);
}

fn render_box<'a, T: 'a>(math_box: MathBox<'a, T>,
                         _: &HarfbuzzShaper,
                         font: &'a FT_Face,
                         output_name: &str) {
    let logical_extents = math_box.bounds().extents;

    let mut document = Document::new();
    // let mut group = Group::new();
    document.assign("viewBox",
                    (math_box.origin.x - 10,
                     math_box.origin.y - logical_extents.ascent - 10,
                     logical_extents.width + 20,
                     logical_extents.descent + logical_extents.ascent + 20));

    let mut ink_group = Group::new().set("stroke", "none").set("fill", "#FFE6E6");
    let mut logical_group = Group::new()
        .set("stroke", "#FF0000")
        .set("stroke-width", 5)
        .set("fill", "none");

    let mut italic_cor_group = Group::new()
        .set("stroke", "black")
        .set("stroke-width", 5)
        .set("fill", "none")
        .set("stroke-dasharray", "30,20")
        .set("stroke-linecap", "round");

    let mut top_accent_attachment_group = Group::new()
        .set("stroke", "green")
        .set("stroke-width", 13)
        .set("fill", "none")
        .set("stroke-dasharray", "140,70")
        .set("stroke-linecap", "round");

    let mut black_group = Group::new().set("fill", "black").set("stroke", "none");

    generate_svg(&mut ink_group,
                 &math_box,
                 &|group, math_box| draw_ink_rect(group, math_box));
    generate_svg(&mut logical_group,
                 &math_box,
                 &|group, math_box| draw_logical_bounds(group, math_box));
    generate_svg(&mut italic_cor_group,
                 &math_box,
                 &|group, math_box| draw_italic_correction(group, math_box));
    generate_svg(&mut top_accent_attachment_group,
                 &math_box,
                 &|group, math_box| draw_top_accent_attachment(group, math_box));
    generate_svg(&mut black_group,
                 &math_box,
                 &|group, math_box| draw_glyph(group, math_box, font));
    generate_svg(&mut black_group,
                 &math_box,
                 &|group, math_box| draw_filled(group, math_box));

    // document.append(ink_group);
    // document.append(logical_group);
    // document.append(italic_cor_group);
    document.append(black_group);
    // document.append(top_accent_attachment_group);

    // let baseline = Line::new()
    //     .set("stroke", "green")
    //     .set("stroke-width", 8)
    //     .set("x1", origin.x - 10)
    //     .set("x2", logical_extents.width + 10)
    //     .set("y1", 0)
    //     .set("y2", 0)
    //     .set("stroke-dasharray", "30,20")
    //     .set("stroke-linecap", "round");
    // group.append(baseline);
    // document.append(group);
    // document.append(baseline);

    svg::save(output_name, &document).unwrap();
}


fn generate_svg<'a, F, U: 'a>(node: &mut Group, math_box: &MathBox<'a, U>, func: &F)
    where F: Fn(&mut Group, &MathBox<'a, U>)
{
    let content = math_box.content();
    match content {
        MathBoxContent::Boxes(list) => {
            let pt = math_box.origin;
            if pt.x == 0 && pt.y == 0 {
                for item in list {
                    generate_svg(node, item, func);
                }
                return;
            }
            let mut group = Group::new()
                .set("transform", format!("translate({:?}, {:?})", pt.x, pt.y));
            for item in list {
                generate_svg(&mut group, item, func);
            }
            node.append(group);
        }
        _ => func(node, math_box),
    }
}

fn draw_filled<'a, T: Node, U: 'a>(doc: &mut T, math_box: &MathBox<'a, U>) {
    if let MathBoxContent::Line { vector, thickness } = math_box.content() {
        let line = Line::new()
            .set("x1", math_box.origin.x)
            .set("y1", math_box.origin.y - math_box.ascent())
            .set("x2", vector.x + math_box.origin.x)
            .set("y2", math_box.origin.y - math_box.ascent() + vector.y)
            .set("stroke-width", thickness)
            .set("stroke", "black")
            .set("z-index", 1);

        doc.append(line);
    }
    if let MathBoxContent::Empty = math_box.content() {
        let rect = Rectangle::new()
            .set("x", math_box.origin.x)
            .set("y", math_box.origin.y - math_box.ascent())
            .set("width", math_box.width())
            .set("height", 100)
            .set("stroke", "none")
            .set("fill", "red")
            .set("z-index", 1);

        // doc.append(rect);
    }
}

fn draw_ink_rect<'a, T: Node, U: 'a>(group: &mut T, math_box: &MathBox<'a, U>) {
    if let MathBoxContent::Glyph(_) = math_box.content() {

        let ink_bounds = math_box.bounds();

        let ink_rect = Rectangle::new()
            .set("x", ink_bounds.origin.x)
            .set("y", ink_bounds.origin.y - ink_bounds.extents.ascent)
            .set("width", ink_bounds.extents.width)
            .set("height",
                 ink_bounds.extents.ascent + ink_bounds.extents.descent);

        group.append(ink_rect);
    }
}

fn draw_logical_bounds<'a, T: Node, U: 'a>(group: &mut T, math_box: &MathBox<'a, U>) {
    if let MathBoxContent::Glyph(_) = math_box.content() {
        let logical_bounds = math_box.bounds().normalize();

        if logical_bounds.extents.ascent != 0 {
            let logical_rect1 = Rectangle::new()
                .set("x", logical_bounds.origin.x)
                .set("y", logical_bounds.origin.y - logical_bounds.extents.ascent)
                .set("width", logical_bounds.extents.width)
                .set("height", logical_bounds.extents.ascent);
            group.append(logical_rect1);
        }

        if logical_bounds.extents.descent != 0 {
            let logical_rect2 = Rectangle::new()
                .set("x", logical_bounds.origin.x)
                .set("y", logical_bounds.origin.y)
                .set("width", logical_bounds.extents.width)
                .set("height", logical_bounds.extents.descent);
            group.append(logical_rect2);
        }
    }
}

fn draw_italic_correction<'a, T: Node, U: 'a>(doc: &mut T, math_box: &MathBox<'a, U>) {
    if let MathBoxContent::Glyph(_) = math_box.content() {
        let ink_bounds = math_box.bounds().normalize();

        if math_box.italic_correction() == 0 {
            return;
        }

        let mut group = Group::new().set("transform",
                                         format!("matrix(1 0 {:?} 1 0 0)",
                                                 -math_box.italic_correction() as f32 /
                                                 math_box.height() as f32));

        let ink_rect = Rectangle::new()
            .set("x", ink_bounds.origin.x)
            .set("y", ink_bounds.origin.y - ink_bounds.extents.ascent)
            .set("width",
                 ink_bounds.extents.width - math_box.italic_correction())
            .set("height",
                 ink_bounds.extents.ascent + ink_bounds.extents.descent);

        group.append(ink_rect);

        doc.append(group);
    }
}

fn draw_top_accent_attachment<'a, T: Node, U: 'a>(doc: &mut T, math_box: &MathBox<'a, U>) {
    let line = Line::new()
        .set("x1", math_box.top_accent_attachment() + math_box.origin.x)
        .set("y1", math_box.origin.y + math_box.descent() + 200)
        .set("x2", math_box.top_accent_attachment() + math_box.origin.x)
        .set("y2", math_box.origin.y - math_box.ascent() - 200);
    doc.append(line);
}

fn draw_glyph<'a, T: Node, U: 'a>(doc: &mut T, math_box: &MathBox<'a, U>, face: &FT_Face) {
    let (glyph, scale_x, scale_y) = if let MathBoxContent::Glyph(Glyph { glyph_code, scale }) =
        math_box.content() {
        (glyph_code, scale.horiz.as_scale_mult(), scale.vert.as_scale_mult())
    } else {
        return;
    };



    let mut group = Group::new();
    {
        let origin = math_box.origin;

        face.load_glyph(glyph, face::NO_SCALE).unwrap();
        let outline = face.glyph().outline().unwrap();

        group.assign("transform",
                     format!("translate({:?}, {:?}) scale({:?}, {:?})",
                             origin.x,
                             origin.y,
                             scale_x,
                             -scale_y));

        let mut data = Data::new();
        for contour in outline.contours_iter() {
            let Vector { x, y } = *contour.start();
            data = data.move_to((x, y));
            for curve in contour {
                match curve {
                    Curve::Line(pt) => data = data.line_to((pt.x, pt.y)),
                    Curve::Bezier2(pt1, pt2) => {
                        data = data.quadratic_curve_to((pt1.x, pt1.y, pt2.x, pt2.y))
                    }
                    Curve::Bezier3(pt1, pt2, pt3) => {
                        data = data.cubic_curve_to((pt1.x, pt1.y, pt2.x, pt2.y, pt3.x, pt3.y))
                    }
                }
            }
        }
        data = data.close();
        let path = Path::new().set("d", data);
        group.append(path);
    }

    // let desc_text = format!("Glyph code: {:?}, name: {:?}", glyph, font.get_glyph_name(glyph) );
    // let desc = Description::new().add(Text::new(desc_text));
    // doc.append(desc);
    doc.append(group);
}
