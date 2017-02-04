// includes a generated list of xml entity names and their replacement characters named ENTITIES.
include!(concat!(env!("OUT_DIR"), "/entities.rs"));

use std;
use super::{Result, ParsingError};
use std::borrow::Cow;

enum StrOrChr {
    Str(&'static str),
    Chr(char),
}

impl StrOrChr {
    fn len(&self) -> usize {
        match *self {
            StrOrChr::Str(ref text) => text.len(),
            StrOrChr::Chr(_) => 4,
        }
    }
}

pub fn unescape(s: &str) -> Result<Cow<str>> {
    let mut escapes = Vec::new();
    'outer: for ent_ref in s.split('&').skip(1) {
        if let Some(i) = ent_ref.find(';') {
            let start_index = ent_ref.as_ptr() as usize - s.as_ptr() as usize;
            if ent_ref.as_bytes()[0] == b'#' {
                let replacement = parse_numeric_entity(&ent_ref[1..i])?;
                escapes.push((start_index - 1..start_index + i, StrOrChr::Chr(replacement)));
                continue 'outer;
            }
            for &(name, replacement) in ENTITIES.iter() {
                if &ent_ref[0..i] == name {
                    escapes.push((start_index - 1..start_index + i, StrOrChr::Str(replacement)));
                    continue 'outer;
                }
            }
            return Err(ParsingError::from("unrecognized entity"));
        } else {
            return Err(ParsingError::from("bad entity"));
        }
    }
    if escapes.is_empty() {
        Ok(Cow::Borrowed(s))
    } else {
        let len = escapes.iter().fold(s.len(),
                                      |acc, &(_, ref replacement)| acc + replacement.len());
        let mut res = String::with_capacity(len);
        let mut start = 0;
        for (range, replacement) in escapes {
            res.push_str(&s[start..range.start]);
            match replacement {
                StrOrChr::Str(text) => res.push_str(text),
                StrOrChr::Chr(chr) => res.push(chr),
            }
            start = range.end + 1;
        }
        if start < s.len() {
            res.push_str(&s[start..]);
        }
        Ok(Cow::Owned(res))
    }
}

fn parse_numeric_entity(ent: &str) -> Result<char> {
    match ent {
        "" => Err(ParsingError::from("empty entity")),
        "x0" | "0" => Err(ParsingError::from("malformed entity")),
        ent => {
            let bytes = ent.as_bytes();
            if bytes[0] == b'x' {
                let name = &ent[1..];
                match u32::from_str_radix(name, 16).ok().and_then(std::char::from_u32) {
                    Some(c) => Ok(c),
                    None => {
                        Err(ParsingError::from("Invalid hexadecimal character number in an entity"))
                    }
                }
            } else {
                let name = &ent[..];
                match u32::from_str_radix(name, 10).ok().and_then(std::char::from_u32) {
                    Some(c) => Ok(c),
                    None => {
                        Err(ParsingError::from("Invalid decimal character number in an entity"))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape() {
        assert_eq!("Hello World!", unescape("Hello World!").unwrap());
        assert_eq!("Hello World#", unescape("Hello World&num;").unwrap());
        assert_eq!("Hello#World", unescape("Hello&num;World").unwrap());
        assert_eq!("#Hello World", unescape("&num;Hello World").unwrap());
        assert_eq!("#HelloÄWorld", unescape("&num;Hello&Auml;World").unwrap());

        assert_eq!("Hello World!", unescape("Hello World&#x21;").unwrap());
        assert_eq!("Hello World!", unescape("Hello World&#33;").unwrap());
    }
}
