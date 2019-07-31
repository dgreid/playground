extern crate program_config_derive;

use program_config_derive::ConfigStruct;

#[derive(Default, ConfigStruct)]
struct Foo {
    #[required = "false"]
    a: u32,
    #[required = "true"]
    b: u32,
}

fn main() {
    let foo = Foo { a: 3, b: 4 };

    println!("foo.a = {} {}", foo.a, foo.get_b());
}
