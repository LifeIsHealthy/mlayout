#[macro_use]
extern crate nom;
use nom::{digit, hex_digit};
use std::str::FromStr;

#[derive(Debug)]
enum Form {
    Prefix,
    Infix,
    Postfix,
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
enum Flag {
    SYMMETRIC,
    FENCE,
    STRETCHY,
    SEPARATOR,
    ACCENT,
    LARGEOP,
    MOVABLE_LIMITS,
}

#[derive(Debug)]
struct Line {
    charcode: i32,
    form: Form,
    lspace: u8,
    rspace: u8,
    flags: Vec<Flag>
}

impl Line {
    fn print_entry(self) {
        let flags = if self.flags.is_empty() {
            "NONE".to_string()
        } else {
            let mut flags = self.flags.into_iter();
            let mut flg_string = format!("{:?}", flags.next().unwrap());
            for additional_flag in flags {
                flg_string += &format!(" | {:?}", additional_flag);
            }
            flg_string
        };
        print!("Entry {{ character: '\\u{{{:X}}}', form: Form::{:?}, lspace: {:?}, rspace: {:?}, flags: {:} }}", self.charcode, self.form, self.lspace, self.rspace, flags);
    }
}

named!(u8_parse<u8>, map_res!(map_res!(digit, std::str::from_utf8), u8::from_str));
named!(hex_parse<i32>, preceded!(tag!("0x"), map_res!(
    map_res!(hex_digit, std::str::from_utf8), |string| i32::from_str_radix(string, 16)
)));
named!(form_parse<Form>, alt!(map!(tag!("Prefix"), |_| Form::Prefix) |
                              map!(tag!("Postfix"), |_| Form::Postfix) |
                              map!(tag!("Infix"), |_| Form::Infix)));

named!(choose_flag<Flag>, alt!(
    map!(tag!("Symmetric"), |_| Flag::SYMMETRIC) |
    map!(tag!("Fence"), |_| Flag::FENCE) |
    map!(tag!("Stretchy"), |_| Flag::STRETCHY) |
    map!(tag!("Separator"), |_| Flag::SEPARATOR) |
    map!(tag!("Accent"), |_| Flag::ACCENT) |
    map!(tag!("LargeOp"), |_| Flag::LARGEOP) |
    map!(tag!("MovableLimits"), |_| Flag::MOVABLE_LIMITS)
));
named!(flag_parse<Vec<Flag>>, ws!(alt!(map!(char!('0'), |_| Vec::new()) |
                            separated_list!(
                                char!('|'),
                                choose_flag
                            )
)));

named!(line_parse<Line>, ws!(delimited!(char!('{'),
    do_parse!(
        hex: hex_parse >>
        char!(',') >>
        form: form_parse >>
        char!(',') >>
        lspace: u8_parse >>
        char!(',') >>
        rspace: u8_parse >>
        char!(',') >>
        flags: flag_parse >>
        (Line { charcode: hex, form: form, lspace: lspace as u8, rspace: rspace as u8, flags: flags })
    ),
    char!('}'))));

named!(parse<Vec<Line>>, ws!(separated_list!(char!(','), do_parse!(
    take_until!("{") >>
    line: line_parse >>
    (line)
))));

fn main() {
    let bytes = include_bytes!("table.txt");
    for line in parse(bytes).unwrap().1 {
        line.print_entry();
        println!(",");
    }
}
