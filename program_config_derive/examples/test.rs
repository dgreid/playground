extern crate program_config_derive;

use program_config_derive::ConfigStruct;

#[derive(Default, ConfigStruct)]
struct Config {
    #[required = "false"]
    #[parse {|a: &str| {println!("val string {}", a);a.parse()}}]
    all: u32,
    #[required = "true"]
    #[parse {|a: &str| {println!("val string {}", a);a.parse()}}]
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
