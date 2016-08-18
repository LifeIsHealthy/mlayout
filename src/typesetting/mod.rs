extern crate freetype;

pub use types::*;

use std::iter::*;
use std::rc::Rc;
use std::cell::{Cell, RefCell};


macro_rules! ot_tag {
    ($t1:expr, $t2:expr, $t3:expr, $t4:expr) => (
        (($t1 as u32) << 24) | (($t2 as u32) << 16) | (($t3 as u32) << 8) | ($t4 as u32)
    );
}

mod layout;
pub mod font;
mod shaper;
pub mod math_box;
mod multiscripts;
mod unicode_math;

use self::layout::{MathBoxLayout, ListIter, BoxIter, LayoutOptions};
use self::font::{MathFont};
use self::shaper::MathShaper;
pub use self::math_box::{MathBox, Content, Bounds, Extents, Point};

pub use self::unicode_math::{Family, convert_character_to_family};

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn list_to_boxes(list: List) -> MathBox {
    let bytes = include_bytes!("../../tests/testfiles/latinmodern-math.otf");
    let library = freetype::Library::init().unwrap();
    let options = LayoutOptions {
        font: MathFont::from_bytes(bytes, 0, &library),
        shaper: Rc::new(RefCell::new(MathShaper::new())),
        style: MathStyle::DisplayStyle,

        ft_library: Rc::new(library),

        //preprocessor: Rc::new(join_atoms),
    };

    let iter = list.into_iter();
    iter.layout(options).collect()
}

fn join_atoms(iter: ListIter) -> ListIter {
    Box::new(iter)
    //Box::new(AtomJoiner { iter: iter.peekable() })
}

struct AtomJoiner {
    iter: Peekable<ListIter>,
}
impl Iterator for AtomJoiner {
    type Item = ListItem;
    fn next(&mut self) -> Option<ListItem> {
        let mut cur_item = match self.iter.next() {
            Some(item) => item,
            None => return None,
        };

        let mut string = String::new();
        let mut should_finalize = false;
        loop {
            match cur_item {
                ListItem::Atom(ref atom @ Atom{
                                nucleus: Field::Unicode(..), ..
                            }) if !atom.has_any_attachments() => {
                    let next_atom = self.iter.peek();
                    match next_atom {
                        Some(&ListItem::Atom(ref next_atom @ Atom{
                                    nucleus: Field::Unicode(..), ..
                                })) if !next_atom.has_any_attachments() => {},
                        _ => should_finalize = true,
                    }
                }
                _ => return Some(cur_item),
            };

            let content = match cur_item {
                ListItem::Atom(Atom { nucleus: Field::Unicode(txt), .. }) => txt,
                _ => unreachable!(),
            };
            string.push_str(&content);

            if should_finalize && !string.is_empty() {
                break;
            };
            should_finalize = false;
            cur_item = match self.iter.next() {
                Some(item) => item,
                None => {
                    if string.is_empty() {
                        return None;
                    } else {
                        break;
                    }
                }
            };
        }

        Some(ListItem::Atom(Atom::new_with_nucleus(Field::Unicode(string))))
    }
}
