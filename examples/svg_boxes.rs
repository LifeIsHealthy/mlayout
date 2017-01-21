extern crate svg;
extern crate math_render;
extern crate freetype;

use math_render::*;
use math_render::math_box::*;
use math_render::font::*;

use svg::Document;
use svg::node::element::{Rectangle, Line, Group, Path, Description};
use svg::node::element::path::Data;
use svg::node::{Node, Text};

use freetype::{face, Vector};
use freetype::outline::Curve;

macro_rules! render_test {
    ( $y:expr, $( $x:expr ),+ ) => (
        $(
            let test = include_bytes!(concat!("../tests/testfiles/", $x,".xml"));
            render_mathml(test, $y, concat!("svg_images/", $x, ".svg"));
        )+
    );
}

fn main() {

    // let font = include_bytes!("/Users/mr/Library/Fonts/latinmodern-math.otf");
    let font = include_bytes!("/Library/Fonts/Microsoft/Cambria Math.ttf");
    // let font = include_bytes!("/Users/mr/Library/Fonts/Asana-Math-2.otf");
    // let font = include_bytes!("/Users/mr/Library/Fonts/texgyreschola-math.otf");
    // let font = include_bytes!("/Users/mr/Library/Fonts/xits-math.otf");

    render_test!(font, "schrödinger", "Vaccent", "horizontal_glyphs", "frac", "pythagoras",
        "italic_sup", "math_kern", "root", "sum", "euler", "limit", "stokes");
    // render_test!(font, "pythagoras");

    let library = freetype::Library::init().unwrap();
    let font = MathFont::from_bytes(font, 0, &library);

    let shaper = math_render::shaper::MathShaper::new();
    let math_box = shaper.shape_stretchy::<()>("√",
                             &font,
                             false,
                             6000,
                             Default::default()).into_iter().collect::<MathBox<_>>();


    render_box(math_box, &font, "svg_images/big_root.svg");
}

fn render_mathml(file: &[u8], font_bytes: &[u8], output_name: &str) {
    let list = mathmlparser::parse(&file[..]).expect("invalid parse");

    let library = freetype::Library::init().unwrap();
    let font = MathFont::from_bytes(font_bytes, 0, &library);

    let parsed_box = math_render::layout(list, &font, &library);

    render_box(parsed_box, &font, output_name);
}

fn render_box<T>(math_box: MathBox<T>, font: &MathFont, output_name: &str) {
    let logical_extents = math_box.logical_extents;

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

    generate_svg(&mut ink_group, &math_box, &draw_ink_rect);
    generate_svg(&mut logical_group, &math_box, &draw_logical_bounds);
    generate_svg(&mut italic_cor_group, &math_box, &draw_italic_correction);
    generate_svg(&mut top_accent_attachment_group,
                 &math_box,
                 &draw_top_accent_attachment);
    generate_svg(&mut black_group,
                 &math_box,
                 &|group, math_box| draw_glyph(group, math_box, font));
    generate_svg(&mut black_group, &math_box, &draw_filled);

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


fn generate_svg<F, U>(node: &mut Group, math_box: &MathBox<U>, func: &F)
    where F: Fn(&mut Group, &MathBox<U>)
{
    match math_box.content {
        Content::Boxes(ref list) => {
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

fn draw_filled<T: Node, U>(doc: &mut T, math_box: &MathBox<U>) {
    if let Content::Filled = math_box.content {
        let bounds = math_box.get_logical_bounds();
        let rect = Rectangle::new()
            .set("x", bounds.origin.x)
            .set("y", bounds.origin.y - bounds.extents.ascent)
            .set("width", bounds.extents.width)
            .set("height", bounds.extents.ascent + bounds.extents.descent)
            .set("stroke", "none")
            .set("fill", "black")
            .set("z-index", 1);

        doc.append(rect);
    }
    if let Content::Empty = math_box.content {
        let bounds = math_box.get_logical_bounds();
        let rect = Rectangle::new()
            .set("x", bounds.origin.x)
            .set("y", bounds.origin.y - bounds.extents.ascent)
            .set("width", bounds.extents.width)
            .set("height", 100)
            .set("stroke", "none")
            .set("fill", "red")
            .set("z-index", 1);

        //doc.append(rect);
    }
}

fn draw_ink_rect<T: Node, U>(group: &mut T, math_box: &MathBox<U>) {
    if let Content::Glyph(..) = math_box.content {

        let ink_bounds = math_box.get_ink_bounds();

        let ink_rect = Rectangle::new()
            .set("x", ink_bounds.origin.x)
            .set("y", ink_bounds.origin.y - ink_bounds.extents.ascent)
            .set("width", ink_bounds.extents.width)
            .set("height",
                 ink_bounds.extents.ascent + ink_bounds.extents.descent);

        group.append(ink_rect);
    }
}

fn draw_logical_bounds<T: Node, U>(group: &mut T, math_box: &MathBox<U>) {
    if let Content::Glyph(..) = math_box.content {
        let logical_bounds = math_box.get_logical_bounds().normalize();

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

fn draw_italic_correction<T: Node, U>(doc: &mut T, math_box: &MathBox<U>) {
    if let Content::Glyph(..) = math_box.content {
        let ink_bounds = math_box.get_ink_bounds().normalize();

        if math_box.italic_correction == 0 {
            return;
        }

        let mut group = Group::new().set("transform",
                                         format!("matrix(1 0 {:?} 1 0 0)",
                                                 -math_box.italic_correction as f32 /
                                                 math_box.ink_extents.height() as f32));

        let ink_rect = Rectangle::new()
            .set("x", ink_bounds.origin.x)
            .set("y", ink_bounds.origin.y - ink_bounds.extents.ascent)
            .set("width",
                 ink_bounds.extents.width - math_box.italic_correction)
            .set("height",
                 ink_bounds.extents.ascent + ink_bounds.extents.descent);

        group.append(ink_rect);

        doc.append(group);
    }
}

fn draw_top_accent_attachment<T: Node, U>(doc: &mut T, math_box: &MathBox<U>) {
    let line = Line::new()
        .set("x1", math_box.top_accent_attachment + math_box.origin.x)
        .set("y1",
             math_box.origin.y + math_box.logical_extents.descent + 200)
        .set("x2", math_box.top_accent_attachment + math_box.origin.x)
        .set("y2",
             math_box.origin.y - math_box.logical_extents.ascent - 200);
    doc.append(line);
}

fn draw_glyph<T: Node, U>(doc: &mut T, math_box: &MathBox<U>, font: &MathFont) {
    let (glyph, scale_x, scale_y) =
        if let MathBox { content: Content::Glyph(Glyph { glyph_code, scale }), .. } =
               *math_box {
            (glyph_code, scale.horiz.as_scale_mult(), scale.vert.as_scale_mult())
        } else {
            return;
        };



    let mut group = Group::new();
    {
        let face = font.ft_face.borrow();
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

    let desc_text = format!("Glyph code: {:?}, name: {:?}", glyph, font.get_glyph_name(glyph) );
    let desc = Description::new().add(Text::new(desc_text));
    doc.append(desc);
    doc.append(group);
}
