extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{self, Span};
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Expr, Field, Ident, Lit, Meta, Type};

// Returns an iterator over the struct's field's names converted to strings.
fn get_long_options(data: &DataStruct) -> impl Iterator<Item = String> + '_ {
    data.fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|ident| ident.to_string()))
}

// Returns an iterator over the struct's fields that are arguments as opposed to flags.
fn arguments(data: &DataStruct) -> impl Iterator<Item = &Field> + '_ {
    data.fields.iter().filter(|field| {
        field
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident(Ident::new("parse", Span::call_site())))
            .is_some()
    })
}

// Returns the type of each field that belongs to the struct.
fn field_types(data: &DataStruct) -> impl Iterator<Item = &Type> + '_ {
    data.fields.iter().map(|field| &field.ty)
}

// Returns an iterator over the struct's fields that are flags as opposed to arguments.
fn flags(data: &DataStruct) -> impl Iterator<Item = &Field> + '_ {
    data.fields.iter().filter(|field| {
        field
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident(Ident::new("flag", Span::call_site())))
            .is_some()
    })
}

fn argument_long_options(data: &DataStruct) -> impl Iterator<Item = String> + '_ {
    arguments(data).filter_map(|f| f.ident.as_ref().map(|ident| ident.to_string()))
}

fn flag_long_options(data: &DataStruct) -> impl Iterator<Item = String> + '_ {
    flags(data).filter_map(|f| f.ident.as_ref().map(|ident| ident.to_string()))
}

// Used to generate the `quote!`ed `HasArg` option to be used with getopts for each of the struct's
// fields.
fn has_args(data: &DataStruct) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    data.fields.iter().map(|field| {
        if field
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident(Ident::new("parse", Span::call_site())))
            .is_some()
        {
            quote!(getopts::HasArg::Yes)
        } else {
            quote!(getopts::HasArg::No)
        }
    })
}

#[proc_macro_derive(ConfigStruct, attributes(flag, parse, required))]
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

    let argument_idents = arguments(&data).filter_map(|f| f.ident.as_ref());
    let flag_idents = flags(&data).filter_map(|f| f.ident.as_ref());
    let member_idents = data.fields.iter().filter_map(|f| f.ident.as_ref());
    let member_types = field_types(&data);
    let long_options = get_long_options(&data);
    let arg_long_options = argument_long_options(&data);
    let flag_long_options = flag_long_options(&data);

    let accessor_names = data
        .fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .map(|n| {
            let concatenated = format!("get_{}", n);
            syn::Ident::new(&concatenated, n.span())
        });
    let has_args = has_args(&data);

    let parsers = arguments(&data).map(|field| {
        syn::parse2::<Expr>(
            field
                .attrs
                .iter()
                .find(|attr| attr.path.is_ident(Ident::new("parse", Span::call_site())))
                .unwrap() // arguments are guaranteed to have a parse field by definition.
                .tts
                .clone(),
        )
        .unwrap()
    });

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

    enum ConfigError {
        ParsingArgs,
    }

    impl #struct_name {
        #(
            fn #accessor_names(&self) -> #member_types {self.#member_idents}
         )*

        pub fn from_args<T>(args: T) -> std::result::Result<#struct_name, ConfigError>
            where
                T: IntoIterator,
                T::Item: AsRef<std::ffi::OsStr>
            {
                let mut cfg = Self::default();

                let opt_parser = build_options_parser();
                let matches = opt_parser.parse(args).map_err(|_| ConfigError::ParsingArgs)?;
                if matches.opt_present("h") {
                    let brief = format!("Usage: TODO [options]");
                    print!("{}", opt_parser.usage(&brief));
                    std::process::exit(0);
                }

                // Set each option if it is specified.
                #(
                    let opt_name = #arg_long_options;
                    if matches.opt_present(opt_name) {
                        let values = matches.opt_strs(opt_name);
                        cfg.#argument_idents = #parsers(&values[0]).unwrap(); // TODO handle parse int
                    }
                )*

                // And flags
                #(
                    let opt_name = #flag_long_options;
                    if matches.opt_present(opt_name) {
                        let values = matches.opt_strs(opt_name);
                        cfg.#flag_idents = true;
                    }
                )*

                Ok(cfg)
            }
        }

        fn build_options_parser() -> getopts::Options {
            let mut options_parser = getopts::Options::new();
            options_parser.optflag("h", "help", "Print this help menu");

            #(
                options_parser.opt(
                    "",// short_names
                    #long_options, // long argument
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
