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

    pub fn arg_parsers(&self) -> impl Iterator<Item = &Expr> {
        self.items.iter().map(|item| &item.arg_parser)
    }

    pub fn var_types(&self) -> impl Iterator<Item = &Box<Type>> {
        self.items.iter().map(|item| &item.var_type)
    }
}

impl Parse for ConfigStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ConfigStruct {
            items: Punctuated::parse_terminated(input)?,
        })
    }
}

// All the information about a particular configuration item.
struct ConfigItem {
    var_type: Box<Type>,
    name: Ident,
    default_val: Expr,
    arg_parser: Expr, // Parses the config value based on the passed argument.
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

enum ItemOption {
    Name(Ident),
    //LongOpt(String),
    //ShortOpt(String),
    Def(Expr),
    VarType(Box<Type>),
    Parser(Expr),
}

impl Parse for ItemOption {
    fn parse(input: ParseStream) -> Result<Self> {
        let tag: Ident = input.parse()?;
        let _: Token![:] = input.parse()?;
        match tag.to_string().as_ref() {
            "NAME" => {
                let name = input.parse()?;
                Ok(ItemOption::Name(name))
            }
            "DEFAULT" => {
                let def = input.parse()?;
                Ok(ItemOption::Def(def))
            }
            "PARSE" => {
                let parser = input.parse()?;
                Ok(ItemOption::Parser(parser))
            }
            "TYPE" => {
                let var_type: Box<Type> = input.parse()?;
                Ok(ItemOption::VarType(var_type))
            }
            _ => panic!("foo"), //Err(()),
        }
    }
}

impl Parse for ConfigItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _paren_token: token::Paren = parenthesized!(content in input);
        let opts: Punctuated<ItemOption, Token![,]> =
            content.parse_terminated(ItemOption::parse)?;

        let mut name = None;
        let mut default_val = None;
        let mut parser = None;
        let mut var_type = None;
        for opt in opts {
            match opt {
                ItemOption::Name(n) => name = Some(n),
                ItemOption::Def(d) => default_val = Some(d),
                ItemOption::Parser(p) => parser = Some(p),
                ItemOption::VarType(v) => var_type = Some(v),
            }
        }

        Ok(ConfigItem {
            name: name.unwrap(),
            default_val: default_val.unwrap(),
            arg_parser: parser.unwrap(),
            var_type: var_type.unwrap(),
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
    let parsers = input.arg_parsers();

    let parser_names = input.names().map(|n| {
        let concatenated = format!("parse_{}", n);
        syn::Ident::new(&concatenated, n.span())
    });
    let parser_names2 = input.names().map(|n| {
        let concatenated = format!("parse_{}", n);
        syn::Ident::new(&concatenated, n.span())
    });
    let parser_names3 = input.names().map(|n| {
        let concatenated = format!("parse_{}", n);
        syn::Ident::new(&concatenated, n.span())
    });
    let types = input.var_types();

    let expanded = quote! {
        struct Config {
            #(#members),*,
            #(#parser_names: Box<dyn Fn(&str, &Config) -> #types>),*
        }

        impl Default for Config {
            fn default() -> Self {
                Config {
                    #(#defaults),*,
                    #(#parser_names2: Box::new(#parsers)),*
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

                let opt_parser = build_options_parser();
                let matches = match opt_parser.parse(args) {
                    Ok(m) => m,
                    Err(e) => {
                        // todo - handle error.
                        return cfg;
                    }
                };
                if matches.opt_present("h") {
                    let brief = format!("Usage: TODO [options]");
                    print!("{}", opt_parser.usage(&brief));
                    std::process::exit(0);
                }

                // Set each option if it is specified.
                #(
                    let opt_name = #option_names;
                    if matches.opt_present(opt_name) {
                        let values = matches.opt_strs(opt_name);
                        if values.len() == 1 { // TODO - handle multiple instances
                            for value in values {
                                cfg.#names = (cfg.#parser_names3)(&value, &cfg);
                            }
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
