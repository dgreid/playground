extern crate program_config;
use program_config::create_config;
create_config!(
    test_val: {
        long_opt: "value",
        arg_type: u32,
        help: "The value to print",
        hint: "INT",
        parse: |values, cfg| {
            // guaranteed there is at least one element in the array.
            let val = values.get(0).unwrap().parse().unwrap();
            if cfg.limit {
                std::cmp::min(val, 100)
            } else {
                val
            }
        }
    },
    max: {
        long_opt: "max",
        short_opt: "m",
        arg_type: u32,
        default: 10,
        help: "The max value",
        hint: "INT",
        parse: |values, _| {
            // guaranteed there is at least one element in the array.
            let val = values.get(0).unwrap().parse().unwrap();
            val
        }
    },
    limit: {
        long_opt: "limit",
        short_opt: "l",
        help: "If specified, limit the value",
    },
);

fn main() {
    let mut args = std::env::args();
    if args.next().is_none() {
        println!("expected executable name");
        return;
    }

    let c = match 
        Config::from_args(args) {
            Ok(c) => c,
            Err(e) => panic!("parsing config {}", e),
        };
    println!(
        "value: {} {}limited",
        c.get_test_val(),
        if c.has_limit() { "" } else { "un" }
    );
}
