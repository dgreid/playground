extern crate program_config_derive;

use program_config_derive::ConfigStruct;

pub fn parse_u32(str_in: &str) -> u32 {
    str_in.parse().unwrap()
}

#[derive(Default, ConfigStruct)]
struct Foo {
    #[required = "false"]
    #[parse = "parse_u32"]
    a: u32,
    #[required = "true"]
    #[parse = "parse_u32"]
    b: u32,
}

fn main() {
    let foo = Foo { a: 3, b: 4 };

    println!("foo.a = {} {}", foo.a, foo.get_b());
}
