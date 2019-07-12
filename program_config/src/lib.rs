///
///
/// ```
/// extern crate program_config;
/// use program_config::create_config;
/// create_config!(
/// (foo, u32, 2, { value.parse().unwrap() } ),
/// (bar, bool, Default::default(), { if value.len() > 3 { true } else { false } } ),
/// );
/// fn main() {
///
///    let mut args = std::env::args();
///     if args.next().is_none() {
///         println!("expected executable name");
///         return;
///     }
///                             
///     let c = Config::default();
///     assert_eq!(c.foo, 2u32);
///     assert_eq!(c.bar, false);
///     let c = Config::from_args(args);
///     assert_eq!(c.foo, 2u32);
///     assert_eq!(c.bar, false);
/// }
/// ```
///
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{self, Punct, Spacing};
use quote::quote;
use quote::{ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, parse_macro_input, token, Expr, Ident, Result, Token, Type};

// The entire configuration space.
struct ConfigStruct {
    items: Punctuated<ConfigItem, Token![,]>,
}

impl ConfigStruct {
    pub fn members(&self) -> impl Iterator<Item = ItemDefinition> {
        self.items.iter().map(|item| item.definition())
    }

    pub fn defaults(&self) -> impl Iterator<Item = ItemDefault> {
        self.items.iter().map(|item| item.default())
    }

    pub fn names(&self) -> impl Iterator<Item = &Ident> {
        self.items.iter().map(|item| &item.name)
    }

    pub fn setter_codes(&self) -> impl Iterator<Item = &Expr> {
        self.items.iter().map(|item| &item.setter)
    }
}

// All the information about a particular configuration item.
struct ConfigItem {
    var_type: Box<Type>,
    name: Ident,
    default_val: Expr,
    setter: Expr, // sets the config value based on the passed argument.
}

impl ConfigItem {
    pub fn definition(&self) -> ItemDefinition {
        ItemDefinition {
            name: &self.name,
            var_type: &self.var_type,
        }
    }

    pub fn default(&self) -> ItemDefault {
        ItemDefault {
            name: &self.name,
            val: &self.default_val,
        }
    }
}

// The definition that will go in the struct.
// Used in a temporary list that can be iterated over and have to_tokens called.
struct ItemDefinition<'a> {
    name: &'a Ident,
    var_type: &'a Box<Type>,
}

impl<'a> ToTokens for ItemDefinition<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.name.to_tokens(tokens);
        tokens.append(Punct::new(':', Spacing::Joint));
        self.var_type.to_tokens(tokens);
    }
}

// The default initailizer that will go in Default::default() for the configuration.
// Used in a temporary list that can be iterated over and have to_tokens called.
struct ItemDefault<'a> {
    name: &'a Ident,
    val: &'a Expr,
}

impl<'a> ToTokens for ItemDefault<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.name.to_tokens(tokens);
        tokens.append(Punct::new(':', Spacing::Joint));
        self.val.to_tokens(tokens);
    }
}

impl Parse for ConfigItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ptoken: token::Paren = parenthesized!(content in input);
        let name = content.parse()?;
        let _: Token![,] = content.parse()?;
        let var_type = content.parse()?;
        let _: Token![,] = content.parse()?;
        let default_val = content.parse()?;
        let _: Token![,] = content.parse()?;
        let setter = content.parse()?;
        Ok(ConfigItem {
            var_type,
            name,
            default_val,
            setter,
        })
    }
}

impl Parse for ConfigStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ConfigStruct {
            items: Punctuated::parse_terminated(input)?,
        })
    }
}

#[proc_macro]
pub fn create_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ConfigStruct);
    let members = input.members();
    let defaults = input.defaults();
    let names = input.names();
    let option_names = input.names().map(|n| n.to_string());
    let option_names2 = input.names().map(|n| n.to_string());
    let setters = input.setter_codes();

    let expanded = quote! {
        struct Config {
            #(#members),*
        }

        impl Default for Config {
            fn default() -> Self {
                Config {
                   #(#defaults),*
                }
            }
        }

        impl Config {
            // TODO - actually parse the args.
            pub fn from_args<T>(args: T) -> Config
            where
                T: IntoIterator,
                T::Item: AsRef<std::ffi::OsStr>
            {
                let mut cfg = Self::default();

                let parser = build_options_parser();
                let matches = match parser.parse(args) {
                    Ok(m) => m,
                    Err(e) => {
                        // todo - handle error.
                        return cfg;
                    }
                };
                if matches.opt_present("h") {
                    let brief = format!("Usage: {} [options]", args[0]);
                    print!("{}", parser.usage(&brief));
                    std::process::exit(0);
                }

                // Set each option if it is specified.
                #(
                    let opt_name = #option_names;
                    if matches.opt_present(opt_name) {
                        let values = matches.opt_strs(opt_name);
                        if values.len == 1 { // TODO - handle multiple instances
                            let value = values[0];
                            cfg.#names = #setters;
                        }
                    }
                )*

                cfg
            }
        }

        fn build_options_parser() -> getopts::Options {
            let mut options_parser = getopts::Options::new();
            options_parser.optflag("h", "help", "Print this help menu");

            #(
                options_parser.opt(
                    "",// #short_names
                    #option_names2,
                    "", //option.help,
                    "", //option.hint,
                    getopts::HasArg::Yes, // option.has_arg,
                    getopts::Occur::Optional, //option.occur,
                    );
            )*

            options_parser
        }
    };

    expanded.into()
}
