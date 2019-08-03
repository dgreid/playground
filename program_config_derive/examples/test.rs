extern crate program_config_derive;

use program_config_derive::ConfigStruct;

pub fn parse_u32(str_in: &str) -> u32 {
    str_in.parse().unwrap()
}

#[derive(Default, ConfigStruct)]
struct Config {
    #[required = "false"]
    #[parse {|a| parse_u32(a)}]
    all: u32,
    #[required = "true"]
    #[parse {|a| {println!("val string {}", a);parse_u32(a)}}]
    value: u32,
}

fn main() {
    let mut args = std::env::args();
    if args.next().is_none() {
        println!("expected executable name");
        return;
    }

    let c = match Config::from_args(args) {
        Ok(c) => c,
        Err(_) => panic!("parsing config "),
    };
    println!("value: {} {}limited", c.get_value(), "un");
}
