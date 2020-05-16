use serde_json::Value;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct EntityData<'a> {
    name: &'a str,
    character: &'a str,
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("entities.rs");

    let json = std::str::from_utf8(include_bytes!("resources/htmlmathml.json")).unwrap();
    let data: Value = serde_json::from_str(json).unwrap();
    let data = data.as_object().unwrap();
    let map = data["characters"].as_object().unwrap();

    let mut entities = Vec::new();
    for (key, value) in map {
        let value = value.as_str().unwrap();
        let new_entity = EntityData {
            name: key,
            character: &value,
        };
        entities.push(new_entity);
    }

    let mut f = File::create(&dest_path).unwrap();

    write!(f,
           "pub static ENTITIES: [(&'static str, &'static str); {:?}] = [",
           entities.len())
            .unwrap();
    for EntityData { name, character } in entities {
        write!(f, "({:?}, {:?}),\n", name, character).unwrap();
    }
    write!(f, "];").unwrap();
}
