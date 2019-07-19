extern crate program_config;
use program_config::create_config;
create_config!(
    (NAME: test_val,
     LONG_OPT: "value",
     TYPE: u32,
     DEFAULT: 2,
     PARSE: |values, cfg| {
         // guaranteed there is at least one element in the array.
         let val = values.get(0).unwrap().parse().unwrap();
         if cfg.limit {
             std::cmp::min(val, 100)
         } else {
             val
         }
    }),
    (NAME: limit,
     LONG_OPT: "limit",
     SHORT_OPT: "l",
    ),
);

fn main() {
    let mut args = std::env::args();
    if args.next().is_none() {
        println!("expected executable name");
        return;
    }

    let c = Config::default();
    assert_eq!(c.test_val, 2u32);
    assert_eq!(c.limit, false);
    let c = Config::from_args(args);
    println!(
        "value: {} {}limited",
        c.test_val,
        if c.limit { "" } else { "un" }
    );
}
