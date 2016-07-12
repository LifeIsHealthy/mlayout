#![feature(specialization)]
#![allow(unknown_lints)]

mod types;
mod typesetting;
pub mod mathmlparser;
pub use typesetting::*;


#[cfg(test)]
mod test {
    use typesetting::*;

    #[test]
    fn it_works() {
        //let atom = Atom{atom_type: Default::default(), inner: AtomContents::Fields{nucleus: Field::Unicode(0x65), ..Default::default()}};
        let atom2: Atom = Default::default();
        let list = vec!(ListItem::Atom(atom2));
        list_to_boxes(list);
    }
}
