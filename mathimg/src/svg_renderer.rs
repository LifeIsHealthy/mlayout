use freetype;

use math_render;
use svg;

use std::path;

use math_render::math_box::*;
use math_render::shaper::*;

use self::svg::node::element::path::Data;
use self::svg::node::element::{Group, Line, Path, Rectangle};
use self::svg::node::Node;
use self::svg::Document;

use freetype::face::Face as FT_Face;
use freetype::outline::Curve;
use freetype::{face, Vector};

pub struct Flags {
    pub show_ink_bounds: bool,
    pub show_logical_bounds: bool,
    pub show_top_accent_attachment: bool,
}

pub fn render<'a, T: AsRef<path::Path>>(
    math_box: MathBox,
    _: &HarfbuzzShaper<'_>,
    font: &'a FT_Face<'_>,
    flags: Flags,
    out_path: T,
) {
    let logical_extents = math_box.extents();

    let mut document = Document::new();
    // let mut group = Group::new();
    document.assign(
        "viewBox",
        (
            math_box.origin.x - 10,
            math_box.origin.y - math_box.extents().ascent - 10,
            math_box.advance_width() + 20,
            logical_extents.descent + logical_extents.ascent + 20,
        ),
    );

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

    generate_svg(&mut italic_cor_group, &math_box, &|group, math_box| {
        draw_italic_correction(group, math_box)
    });
    generate_svg(
        &mut top_accent_attachment_group,
        &math_box,
        &|group, math_box| draw_top_accent_attachment(group, math_box),
    );
    generate_svg(&mut black_group, &math_box, &|group, math_box| {
        draw_glyph(group, math_box, font)
    });
    generate_svg(&mut black_group, &math_box, &|group, math_box| {
        draw_filled(group, math_box)
    });

    if flags.show_ink_bounds {
        let mut ink_group = Group::new().set("stroke", "none").set("fill", "#FFE6E6");
        generate_svg(&mut ink_group, &math_box, &|group, math_box| {
            draw_ink_rect(group, math_box)
        });
        document.append(ink_group);
    }

    if flags.show_logical_bounds {
        let mut logical_group = Group::new()
            .set("stroke", "#FF0000")
            .set("stroke-width", 5)
            .set("fill", "none");
        generate_svg(&mut logical_group, &math_box, &|group, math_box| {
            draw_logical_bounds(group, math_box)
        });
        document.append(logical_group);
    }

    //    document.append(italic_cor_group);
    document.append(black_group);

    if flags.show_top_accent_attachment {
        document.append(top_accent_attachment_group);
    }

    svg::save(out_path, &document).unwrap();
}

fn generate_svg<'a, F>(node: &mut Group, math_box: &MathBox, func: &F)
where
    F: Fn(&mut Group, &MathBox),
{
    let content = math_box.content();
    match *content {
        MathBoxContent::Boxes(ref list) => {
            let pt = math_box.origin;
            if pt.x == 0 && pt.y == 0 {
                for item in list.as_slice() {
                    generate_svg(node, item, func);
                }
                return;
            }
            let mut group =
                Group::new().set("transform", format!("translate({:?}, {:?})", pt.x, pt.y));
            for item in list.as_slice() {
                generate_svg(&mut group, item, func);
            }
            node.append(group);
        }
        _ => func(node, math_box),
    }
}

fn draw_filled<'a, T: Node>(doc: &mut T, math_box: &MathBox) {
    if let MathBoxContent::Drawable(Drawable::Line { vector, thickness }) = *math_box.content() {
        let line = Line::new()
            .set("x1", math_box.origin.x)
            .set("y1", math_box.origin.y - math_box.extents().ascent)
            .set("x2", vector.x + math_box.origin.x)
            .set(
                "y2",
                math_box.origin.y - math_box.extents().ascent + vector.y,
            )
            .set("stroke-width", thickness)
            .set("stroke", "black")
            .set("z-index", 1);

        doc.append(line);
    }
    if let MathBoxContent::Empty(_) = *math_box.content() {
        let _rect = Rectangle::new()
            .set("x", math_box.origin.x)
            .set("y", math_box.origin.y - math_box.extents().ascent)
            .set("width", math_box.extents().width)
            .set("height", 100)
            .set("stroke", "none")
            .set("fill", "red")
            .set("z-index", 1);

        // doc.append(rect);
    }
}

fn draw_ink_rect<'a, T: Node>(group: &mut T, math_box: &MathBox) {
    if let MathBoxContent::Drawable(Drawable::Glyph(_)) = *math_box.content() {
        let ink_rect = Rectangle::new()
            .set(
                "x",
                math_box.origin.x + math_box.extents().left_side_bearing,
            )
            .set("y", math_box.origin.y - math_box.extents().ascent)
            .set("width", math_box.extents().width)
            .set("height", math_box.extents().height());

        group.append(ink_rect);
    }
}

fn draw_logical_bounds<'a, T: Node>(group: &mut T, math_box: &MathBox) {
    if let MathBoxContent::Drawable(Drawable::Glyph(_)) = *math_box.content() {
        let logical_bounds = math_box.bounds().normalize();

        if logical_bounds.extents.ascent != 0 {
            let logical_rect1 = Rectangle::new()
                .set("x", logical_bounds.origin.x)
                .set("y", logical_bounds.origin.y - logical_bounds.extents.ascent)
                .set("width", math_box.advance_width())
                .set("height", logical_bounds.extents.ascent);
            group.append(logical_rect1);
        }

        if logical_bounds.extents.descent != 0 {
            let logical_rect2 = Rectangle::new()
                .set("x", logical_bounds.origin.x)
                .set("y", logical_bounds.origin.y)
                .set("width", math_box.advance_width())
                .set("height", logical_bounds.extents.descent);
            group.append(logical_rect2);
        }
    }
}

fn draw_italic_correction<'a, T: Node>(doc: &mut T, math_box: &MathBox) {
    if let MathBoxContent::Drawable(Drawable::Glyph(_)) = *math_box.content() {
        let ink_bounds = math_box.bounds().normalize();

        if math_box.italic_correction() == 0 {
            return;
        }

        let mut group = Group::new().set(
            "transform",
            format!(
                "matrix(1 0 {:?} 1 0 0)",
                -math_box.italic_correction() as f32 / math_box.extents().height() as f32
            ),
        );

        let ink_rect = Rectangle::new()
            .set(
                "x",
                ink_bounds.origin.x + ink_bounds.extents.left_side_bearing,
            )
            .set("y", ink_bounds.origin.y - ink_bounds.extents.ascent)
            .set(
                "width",
                ink_bounds.extents.width - math_box.italic_correction(),
            )
            .set(
                "height",
                ink_bounds.extents.ascent + ink_bounds.extents.descent,
            );

        group.append(ink_rect);

        doc.append(group);
    }
}

fn draw_top_accent_attachment<'a, T: Node>(doc: &mut T, math_box: &MathBox) {
    let line = Line::new()
        .set("x1", math_box.top_accent_attachment() + math_box.origin.x)
        .set("y1", math_box.origin.y + math_box.extents().descent + 200)
        .set("x2", math_box.top_accent_attachment() + math_box.origin.x)
        .set("y2", math_box.origin.y - math_box.extents().ascent - 200);
    doc.append(line);
}

fn draw_glyph<'a, T: Node>(doc: &mut T, math_box: &MathBox, face: &FT_Face<'_>) {
    let (glyph, scale_x, scale_y) =
        if let MathBoxContent::Drawable(Drawable::Glyph(MathGlyph {
            glyph_code, scale, ..
        })) = *math_box.content()
        {
            (glyph_code, scale.as_scale_mult(), scale.as_scale_mult())
        } else {
            return;
        };

    let mut group = Group::new();
    {
        let origin = math_box.origin;

        face.load_glyph(glyph, face::NO_SCALE).unwrap();
        let outline = face.glyph().outline().expect("Glyph has no outline.");

        group.assign(
            "transform",
            format!(
                "translate({:?}, {:?}) scale({:?}, {:?})",
                origin.x, origin.y, scale_x, -scale_y
            ),
        );

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

    doc.append(group);
}
