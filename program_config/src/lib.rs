///
///
/// ```
/// extern crate program_config;
/// use program_config::create_config;
/// create_config!((u32, foo, 2));
/// fn main() {
///     let c = Config { foo: 2u32 };
///     assert_eq!(c.foo, 2u32);
/// }
/// ```
///
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, parse_macro_input, token, Ident, Lit, Result, Token, Type};

struct ConfigItem {
    mem_type: Box<Type>,
    name: Ident,
    val: Lit,
}

impl Parse for ConfigItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let mem_type = input.parse()?;
        let _: Token![,] = input.parse()?;
        let name = input.parse()?;
        let _: Token![,] = input.parse()?;
        let val = input.parse()?;
        Ok(ConfigItem {
            mem_type,
            name,
            val,
        })
    }
}

struct ConfigStruct {
    items: Vec<ConfigItem>,
}

impl Parse for ConfigStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();
        // TODO handle list of items
        let paren_content;
        let _ptoken: token::Paren = parenthesized!(paren_content in input);
        let item: ConfigItem = paren_content.parse()?;
        items.push(item);
        Ok(ConfigStruct { items })
    }
}

#[proc_macro]
pub fn create_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ConfigStruct);
    // TODO - loop over all items.
    let name = input.items[0].name.clone();
    let struct_code = quote! {
        struct Config {
            #name: u32,
        }
    };
    struct_code.into()
}
