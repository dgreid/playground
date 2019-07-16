extern crate program_config;
use program_config::create_config;
create_config!(
    (NAME: foo, TYPE: u32, DEFAULT: 2, |value: &str| {
        value.parse().unwrap()
    }),
    (NAME: bar, TYPE: bool, DEFAULT: Default::default(), |value: &str| {
        if value.len() > 3 {
            true
        } else {
            false
        }
    }),
);

fn main() {
    let mut args = std::env::args();
    if args.next().is_none() {
        println!("expected executable name");
        return;
    }

    let c = Config::default();
    assert_eq!(c.foo, 2u32);
    assert_eq!(c.bar, false);
    let c = Config::from_args(args);
    assert_eq!(c.foo, 2u32);
    assert_eq!(c.bar, false);
}
