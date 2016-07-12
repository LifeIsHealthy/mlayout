pub use types::*;

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn list_to_boxes(list: List) -> MathBox {
    for cur_item in &list {
        println!("Hello {:?}", cur_item);
    }
    MathBox{width:0,height:0,bearing_x:0,bearing_y:0,field:Field::List(list)}
}
