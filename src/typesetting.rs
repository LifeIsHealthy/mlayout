pub use types::*;

pub fn list_to_boxes<'a>(list: &'a List) -> MathBox<'a> {
    for cur_item in list {
        println!("Hello {:?}", cur_item);
    }
    MathBox{width:0,height:0,bearing_x:0,bearing_y:0,field:Field::List(list)}
}
