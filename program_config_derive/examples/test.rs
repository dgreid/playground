extern crate program_config_derive;

use program_config_derive::ConfigStruct;

#[derive(Default, ConfigStruct)]
struct Config {
    #[required = "false"]
    #[parse {|a: &str| -> Result<u32, std::num::ParseIntError> {println!("val string {}", a);a.parse()}}]
    #[help = "unused"]
    all: u32,
    #[required = "true"]
    #[parse {|a: &str| -> Result<u32, std::num::ParseIntError> {println!("val string {}", a);a.parse()}}]
    #[help = "The limit to set."]
    value: u32,
    #[flag]
    #[help = "If present, enforce the limit."]
    limited: bool,
}

fn main() {
    let c = match Config::from_args(std::env::args()) {
        Ok(Some(c)) => c,
        Ok(None) => std::process::exit(0),
        _ => std::process::exit(1),
    };
    println!(
        "value: {} {}limited",
        c.get_value(),
        if c.get_limited() { "" } else { "un" }
    );
}
