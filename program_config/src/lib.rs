extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{self, Span};
use quote::quote;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, parse_macro_input, token, Expr, Ident, LitStr, Result, Token, Type};

// The entire configuration space.
struct ConfigStruct {
    items: Punctuated<ConfigItem, Token![,]>,
}

impl ConfigStruct {
    pub fn defaults(&self) -> impl Iterator<Item = &Expr> {
        self.items.iter().map(|item| &item.default_val)
    }

    pub fn names(&self) -> impl Iterator<Item = &Ident> {
        self.items.iter().map(|item| &item.name)
    }

    pub fn parser_names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.names().map(|n| {
            let concatenated = format!("parse_{}", n);
            syn::Ident::new(&concatenated, n.span())
        })
    }

    pub fn parser_closures(&self) -> impl Iterator<Item = &Expr> {
        self.items.iter().map(|item| &item.parser_closure)
    }

    pub fn var_types(&self) -> impl Iterator<Item = &Box<Type>> {
        self.items.iter().map(|item| &item.var_type)
    }

    pub fn long_options(&self) -> impl Iterator<Item = &LitStr> {
        self.items
            .iter()
            .map(|item| item.long_opt.unwrap_or(LitStr::new("", Span::def_site())))
    }
}

impl Parse for ConfigStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ConfigStruct {
            items: Punctuated::parse_terminated(input)?,
        })
    }
}

impl ToTokens for ConfigStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let defaults = self.defaults();
        let names = self.names();
        let names_definition = self.names();
        let names_default = self.names();
        let long_options = self.long_options();
        let long_options2 = self.long_options();
        let parser_closures = self.parser_closures();
        let parser_names_definition = self.parser_names();
        let parser_names_creation = self.parser_names();
        let parser_names_call = self.parser_names();
        let types = self.var_types();
        let types2 = self.var_types();

        let code = quote! {
            struct Config {
                #(#names_definition: #types),*,
                #(#parser_names_definition: Box<dyn Fn(Vec<String>, &Config) -> #types2>),*
            }

            impl Default for Config {
                fn default() -> Self {
                    Config {
                        #(#names_default: #defaults),*,
                        #(#parser_names_creation: Box::new(#parser_closures)),*
                    }
                }
            }

            impl Config {
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
                        let opt_name = #long_options;
                        if matches.opt_present(opt_name) {
                            let values = matches.opt_strs(opt_name);
                            cfg.#names = (cfg.#parser_names_call)(values, &cfg);
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
                        #long_options2, // long argument
                        "", //option.help,
                        "", //option.hint,
                        getopts::HasArg::Yes, // option.has_arg,
                        getopts::Occur::Optional, //option.occur,
                        );
                )*

                options_parser
            }
        };

        code.to_tokens(tokens)
    }
}

// All the information about a particular configuration item.
struct ConfigItem {
    var_type: Box<Type>,
    name: Ident,
    default_val: Expr,
    parser_closure: Expr, // Parses the config value based on the passed argument.
    long_opt: Option<LitStr>,
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
        let mut long_opt = None;
        for opt in opts {
            match opt {
                ItemOption::Name(n) => name = Some(n),
                ItemOption::Def(d) => default_val = Some(d),
                ItemOption::Parser(p) => parser = Some(p),
                ItemOption::VarType(v) => var_type = Some(v),
                ItemOption::LongOpt(o) => long_opt = Some(o),
            }
        }

        Ok(ConfigItem {
            name: name.unwrap(),
            default_val: default_val.unwrap(),
            parser_closure: parser.unwrap(),
            var_type: var_type.unwrap(),
            long_opt,
        })
    }
}

enum ItemOption {
    Name(Ident),
    LongOpt(LitStr),
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
            "LONG_OPT" => {
                let opt_name: LitStr = input.parse()?;
                Ok(ItemOption::LongOpt(opt_name))
            }
            _ => panic!("foo"), //Err(()),
        }
    }
}

#[proc_macro]
pub fn create_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ConfigStruct);
    let mut tokens = proc_macro2::TokenStream::new();
    input.to_tokens(&mut tokens);
    tokens.into()
}
