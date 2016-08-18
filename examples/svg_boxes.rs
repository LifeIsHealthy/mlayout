extern crate svg;
extern crate math_render;
extern crate freetype;

use math_render::*;

use svg::Document;
use svg::node::element::{Rectangle, Line, Group, Path};
use svg::node::element::path::Data;
use svg::node::Node;

use freetype::{Library, face, Vector};
use freetype::outline::Curve;

fn main() {
    let bytes = include_bytes!("../tests/testfiles/trivial.xml");
    let list = mathmlparser::parse(&bytes[..]).expect("invalid parse");
    let parsed_box = math_render::list_to_boxes(list);

    let origin = parsed_box.origin;
    let logical_extents = parsed_box.logical_extents;

    let mut document = Document::new();
    // let mut group = Group::new();
    document.assign("viewBox",
                    (parsed_box.origin.x - 10,
                     parsed_box.origin.y - logical_extents.ascent - 10,
                     logical_extents.width + 20,
                     logical_extents.descent + logical_extents.ascent + 20));

    // group.assign("transform",
    //              format!("translate(0, {:?}) scale(1, -1) translate(0, {:?})",
    //                      logical_extents.ascent,
    //                      logical_extents.descent));
    // generate_svg(&mut group, parsed_box);

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

    let mut black_group = Group::new().set("fill", "black").set("stroke", "none");

    generate_svg(&mut ink_group, &parsed_box, draw_ink_rect);
    generate_svg(&mut logical_group, &parsed_box, draw_logical_bounds);
    generate_svg(&mut italic_cor_group, &parsed_box, draw_italic_correction);
    generate_svg(&mut black_group, &parsed_box, draw_glyph);
    generate_svg(&mut black_group, &parsed_box, draw_filled);

    document.append(ink_group);
    document.append(logical_group);
    document.append(italic_cor_group);
    document.append(black_group);

    let baseline = Line::new()
        .set("stroke", "green")
        .set("stroke-width", 8)
        .set("x1", origin.x - 10)
        .set("x2", logical_extents.width + 10)
        .set("y1", 0)
        .set("y2", 0)
        .set("stroke-dasharray", "30,20")
        .set("stroke-linecap", "round");
    // group.append(baseline);
    // document.append(group);
    //document.append(baseline);

    svg::save("image.svg", &document).unwrap();
}


fn generate_svg<F>(node: &mut Group, math_box: &MathBox, func: F)
    where F: Fn(&mut Group, &MathBox) + Copy
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

fn draw_filled<T: Node>(doc: &mut T, math_box: &MathBox) {
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
}

fn draw_ink_rect<T: Node>(group: &mut T, math_box: &MathBox) {
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

fn draw_logical_bounds<T: Node>(group: &mut T, math_box: &MathBox) {
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

fn draw_italic_correction<T: Node>(doc: &mut T, math_box: &MathBox) {
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
            .set("width", ink_bounds.extents.width - math_box.italic_correction)
            .set("height",
                 ink_bounds.extents.ascent + ink_bounds.extents.descent);

        group.append(ink_rect);

        doc.append(group);
    }
}

fn draw_glyph<T: Node>(doc: &mut T, math_box: &MathBox) {
    let (glyph, scale_x, scale_y) =
        if let MathBox { content: Content::Glyph(Glyph { glyph_code, scale_x, scale_y }), .. } =
               *math_box {
            (glyph_code, scale_x, scale_y)
        } else {
            return;
        };

    let lib = Library::init().unwrap();
    let face = lib.new_face("/Library/Fonts/latinmodern-math.otf", 0).unwrap();
    let origin = math_box.origin;

    face.load_glyph(glyph, face::NO_SCALE).unwrap();
    let outline = face.glyph().outline().unwrap();
    let x = face.glyph().metrics().horiBearingX as i32;

    let mut group = Group::new();
    let scale_x = (scale_x as f32) / 100f32;
    let scale_y = (scale_y as f32) / 100f32;
    group.assign("transform",
                 format!("translate({:?}, {:?}) scale({:?}, {:?})",
                         origin.x - x,
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
    doc.append(group);
}
