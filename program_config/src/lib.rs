extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{self, Span};
use quote::quote;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, token, Error, Expr, Ident, LitStr, Result, Token, Type};

// The entire configuration space.
struct ConfigStruct {
    items: Punctuated<ConfigItem, Token![,]>,
}

impl ConfigStruct {
    fn option_data(&self) -> impl Iterator<Item = &ConfigOption> {
        self.items.iter().filter_map(|i| {
            if let ConfigType::Opt(data) = &i.config_type {
                Some(data)
            } else {
                None
            }
        })
    }

    fn option_defaults(&self) -> impl Iterator<Item = &Expr> {
        self.option_data().map(|d| &d.default_val)
    }

    fn options(&self) -> impl Iterator<Item = &ConfigItem> {
        self.items.iter().filter(|i| match i.config_type {
            ConfigType::Opt(_) => true,
            _ => false,
        })
    }

    fn flags(&self) -> impl Iterator<Item = &ConfigItem> {
        self.items.iter().filter(|i| match i.config_type {
            ConfigType::Flag => true,
            _ => false,
        })
    }

    fn option_names(&self) -> impl Iterator<Item = &Ident> {
        self.options().map(|item| &item.name)
    }

    fn flag_names(&self) -> impl Iterator<Item = &Ident> {
        self.flags().map(|item| &item.name)
    }

    fn flag_accessors(&self) -> impl Iterator<Item = Ident> + '_ {
        self.flag_names().map(|n| {
            let concatenated = format!("has_{}", n);
            syn::Ident::new(&concatenated, n.span())
        })
    }

    fn option_accessors(&self) -> impl Iterator<Item = Ident> + '_ {
        self.option_names().map(|n| {
            let concatenated = format!("get_{}", n);
            syn::Ident::new(&concatenated, n.span())
        })
    }

    fn parser_names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.option_names().map(|n| {
            let concatenated = format!("parse_{}", n);
            syn::Ident::new(&concatenated, n.span())
        })
    }

    fn parser_closures(&self) -> impl Iterator<Item = &Expr> {
        self.option_data().map(|item| &item.parser_closure)
    }

    fn var_types(&self) -> impl Iterator<Item = &Box<Type>> {
        self.option_data().map(|d| &d.var_type)
    }

    fn long_options(&self) -> impl Iterator<Item = &LitStr> {
        self.options().map(|item| &item.long_opt)
    }

    fn short_options(&self) -> impl Iterator<Item = Option<&LitStr>> {
        self.options().map(|item| item.short_opt.as_ref())
    }

    fn option_helps(&self) -> impl Iterator<Item = Option<&LitStr>> {
        self.options().map(|item| item.help.as_ref())
    }

    fn option_hints(&self) -> impl Iterator<Item = Option<&LitStr>> {
        self.options().map(|item| item.hint.as_ref())
    }

    fn long_flags(&self) -> impl Iterator<Item = &LitStr> {
        self.flags().map(|item| &item.long_opt)
    }

    fn short_flags(&self) -> impl Iterator<Item = Option<&LitStr>> {
        self.flags().map(|item| item.short_opt.as_ref())
    }

    fn flag_helps(&self) -> impl Iterator<Item = Option<&LitStr>> {
        self.flags().map(|item| item.help.as_ref())
    }

    fn flag_hints(&self) -> impl Iterator<Item = Option<&LitStr>> {
        self.flags().map(|item| item.hint.as_ref())
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
        let empty_str = LitStr::new("", Span::call_site());
        let defaults = self.option_defaults();
        let option_names = self.option_names();
        let flag_names = self.flag_names();
        let flag_names2 = self.flag_names();
        let flag_names3 = self.flag_names();
        let long_options = self.long_options();
        let long_options2 = self.long_options();
        let short_options = self.short_options().map(|o| o.unwrap_or(&empty_str));
        let long_flags = self.long_flags();
        let long_flags2 = self.long_flags();
        let short_flags = self.short_flags().map(|o| o.unwrap_or(&empty_str));
        let parser_closures = self.parser_closures();
        let parser_names_definition = self.parser_names();
        let parser_names_creation = self.parser_names();
        let parser_names_call = self.parser_names();
        let option_names2 = self.option_names();
        let option_names3 = self.option_names();
        let option_accessors = self.option_accessors();
        let option_helps = self.option_helps().map(|h| h.unwrap_or(&empty_str));
        let option_hints = self.option_hints().map(|h| h.unwrap_or(&empty_str));
        let names_default = self.option_names();
        let types = self.var_types();
        let types2 = self.var_types();
        let option_types = self.var_types();
        let flag_accessors = self.flag_accessors();
        let flag_names_default = self.flag_names();
        let flag_helps = self.flag_helps().map(|h| h.unwrap_or(&empty_str));
        let flag_hints = self.flag_hints().map(|h| h.unwrap_or(&empty_str));

        let code = quote! {
            struct Config {
                #(
                    #option_names: #types,
                    #parser_names_definition: Box<dyn Fn(Vec<String>, &Config) -> #types2>,
                )*

                #(#flag_names: bool),*
            }

            impl Default for Config {
                fn default() -> Self {
                    Config {
                        #(
                            #names_default: #defaults,
                            #parser_names_creation: Box::new(#parser_closures),
                        )*

                        #(#flag_names_default: false),*
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
                            println!("argument parsing error");
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
                            cfg.#option_names2 = (cfg.#parser_names_call)(values, &cfg);
                        }
                    )*

                    // And flags
                    #(
                        if matches.opt_present(#long_flags) {
                            cfg.#flag_names2 = true;
                        }
                    )*

                    cfg
                }

                // accessors for each option.
                #(
                    pub fn #option_accessors(&self) -> &#option_types {
                        &self.#option_names3
                    }
                )*

                // accessors for each flag.
                #(
                    pub fn #flag_accessors(&self) -> bool {
                        self.#flag_names3
                    }
                )*
            }

            fn build_options_parser() -> getopts::Options {
                let mut options_parser = getopts::Options::new();
                options_parser.optflag("h", "help", "Print this help menu");

                // TODO allow required args vs optional args.
                // need to allow options in the Config class.
                #(
                    options_parser.opt(
                        #short_options,// short_names
                        #long_options2, // long argument
                        #option_helps, //option.help,
                        #option_hints, //option.hint,
                        getopts::HasArg::Yes, // option.has_arg,
                        getopts::Occur::Optional, //option.occur,
                        );
                )*

                #(
                    options_parser.opt(
                        #short_flags,// short_names
                        #long_flags2, // long argument
                        #flag_helps, //option.help,
                        #flag_hints, //option.hint,
                        getopts::HasArg::No, // option.has_arg,
                        getopts::Occur::Optional, //option.occur,
                        );
                )*

                options_parser
            }
        };

        code.to_tokens(tokens)
    }
}

struct ConfigOption {
    var_type: Box<Type>,
    default_val: Expr,
    parser_closure: Expr, // Parses the config value based on the passed argument.
}

enum ConfigType {
    Opt(ConfigOption),
    Flag,
}

// All the information about a particular configuration item.
struct ConfigItem {
    name: Ident,
    long_opt: LitStr,
    short_opt: Option<LitStr>,
    config_type: ConfigType,
    help: Option<LitStr>,
    hint: Option<LitStr>,
}

impl Parse for ConfigItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let name: Ident = input.parse()?;
        let _: Token![:] = input.parse()?;
        let _brace_token: token::Brace = braced!(content in input);
        let spec: Punctuated<ItemOption, Token![,]> =
            content.parse_terminated(ItemOption::parse)?;
        let error_span = name.span();

        let mut default_val = None;
        let mut parser = None;
        let mut var_type = None;
        let mut long_opt = None;
        let mut short_opt = None;
        let mut help = None;
        let mut hint = None;
        for var in spec {
            match var {
                ItemOption::Def(d) => default_val = Some(d),
                ItemOption::Parser(p) => parser = Some(p),
                ItemOption::VarType(v) => var_type = Some(v),
                ItemOption::LongOpt(o) => long_opt = Some(o),
                ItemOption::ShortOpt(o) => short_opt = Some(o),
                ItemOption::Help(h) => help = Some(h),
                ItemOption::Hint(h) => hint = Some(h),
            }
        }

        let long_opt = long_opt.ok_or(
            Error::new(error_span,
            "Long option string is required, specify with `long_opt`")
        )?;

        Ok(ConfigItem {
            name: name,
            long_opt: long_opt,
            short_opt,
            config_type: if let Some(var_type) = var_type {
                ConfigType::Opt(ConfigOption {
                    default_val: default_val.ok_or(Error::new(error_span, "A default value must be given with `default_val` for options with arguments"))?,
                    parser_closure: parser.ok_or(Error::new(error_span, "A parser must be specified with `parse` for options with arguments"))?,
                    var_type: var_type,
                })
            } else {
                ConfigType::Flag
            },
            help,
            hint
        })
    }
}

enum ItemOption {
    Help(LitStr),
    Hint(LitStr),
    LongOpt(LitStr),
    ShortOpt(LitStr),
    Def(Expr),
    VarType(Box<Type>),
    Parser(Expr),
}

impl Parse for ItemOption {
    fn parse(input: ParseStream) -> Result<Self> {
        let tag: Ident = input.parse()?;
        let _: Token![:] = input.parse()?;
        match tag.to_string().as_ref() {
            "default" => {
                let def = input.parse()?;
                Ok(ItemOption::Def(def))
            }
            "parse" => {
                let parser = input.parse()?;
                Ok(ItemOption::Parser(parser))
            }
            "arg_type" => {
                let var_type: Box<Type> = input.parse()?;
                Ok(ItemOption::VarType(var_type))
            }
            "long_opt" => {
                let opt_name: LitStr = input.parse()?;
                Ok(ItemOption::LongOpt(opt_name))
            }
            "short_opt" => {
                let opt_name: LitStr = input.parse()?;
                Ok(ItemOption::ShortOpt(opt_name))
            }
            "help" => {
                let help_str: LitStr = input.parse()?;
                Ok(ItemOption::Help(help_str))
            }
            "hint" => {
                let hint_str: LitStr = input.parse()?;
                Ok(ItemOption::Hint(hint_str))
            }
            _ => Err(Error::new(tag.span(), "Unknown option attribute")),
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
