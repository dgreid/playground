extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Expr, Ident, Lit, Meta};

#[proc_macro_derive(ConfigStruct, attributes(parse, required))]
pub fn config_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    let data = match input.data {
        Data::Struct(d) => d,
        _ => {
            println!("not a struct");
            std::process::exit(0);
        }
    };

    let member_idents = data.fields.iter().filter_map(|f| f.ident.as_ref());
    let member_idents2 = data.fields.iter().filter_map(|f| f.ident.as_ref());
    let long_options = data
        .fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()));
    let long_options2 = data
        .fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()));

    let accessor_names = data
        .fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .map(|n| {
            let concatenated = format!("get_{}", n);
            syn::Ident::new(&concatenated, n.span())
        });
    // TODO allow flags
    let has_args = data
        .fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .map(|_| quote!(getopts::HasArg::Yes));

    let parsers = data
        .fields
        .iter()
        .map(|f| {
            f.attrs
                .iter()
                .find(|attr| attr.path.is_ident(Ident::new("parse", Span::call_site())))
                .unwrap()
        })
        .map(|attr| syn::parse2::<Expr>(attr.tts.clone()).unwrap());

    let is_required = data.fields.iter().map(|f| {
        if f.attrs.iter().any(|a| {
            a.parse_meta()
                .map(|m| {
                    match m {
                        Meta::NameValue(name_value) => {
                            if name_value.ident.to_string() == "required" {
                                if let Lit::Str(lit_str) = name_value.lit {
                                    if lit_str.value() == "true" {
                                        return true;
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                    false
                })
                .unwrap_or(false)
        }) {
            quote!(getopts::Occur::Req)
        } else {
            quote!(getopts::Occur::Optional)
        }
    });

    let expanded = quote! {
        impl #struct_name {
        #(
            fn #accessor_names(&self) -> u32 {self.#member_idents2}
         )*

                pub fn from_args<T>(args: T) -> std::result::Result<#struct_name, ()>
                where
                    T: IntoIterator,
                    T::Item: AsRef<std::ffi::OsStr>
                {
                    let mut cfg = Self::default();

                    let opt_parser = build_options_parser();
                    let matches = opt_parser.parse(args).unwrap();//map_err(ConfigError::ParsingArgs)?;
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
                            cfg.#member_idents = #parsers(&values[0]).unwrap(); // TODO handle parse int
                        }
                    )*

                    // TODO And flags

                    Ok(cfg)
                }

        }
            fn build_options_parser() -> getopts::Options {
                let mut options_parser = getopts::Options::new();
                options_parser.optflag("h", "help", "Print this help menu");

                // TODO allow required args vs optional args.
                // need to allow options in the Config class.
                #(
                    options_parser.opt(
                        "",// short_names
                        #long_options2, // long argument
                        "", //option.help,
                        "", //option.hint,
                        #has_args, //option.has_arg,
                        #is_required, //option.occur,
                        );
                )*

                options_parser
            }
    };

    TokenStream::from(expanded)
}
