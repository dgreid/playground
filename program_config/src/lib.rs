///
///
/// ```
/// extern crate program_config;
/// use program_config::create_config;
/// create_config!((u32, foo, 2));
/// fn main() {
///     let c = Config::default();
///     assert_eq!(c.foo, 2u32);
/// }
/// ```
///
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{self, Punct, Spacing};
use quote::quote;
use quote::{ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, parse_macro_input, token, Expr, Ident, Result, Token, Type};

// The entire configuration space.
struct ConfigStruct {
    items: Vec<ConfigItem>,
}

impl ConfigStruct {
    pub fn members(&self) -> impl Iterator<Item = ItemDefinition> {
        self.items.iter().map(|item| item.definition())
    }

    pub fn defaults(&self) -> impl Iterator<Item = ItemDefault> {
        self.items.iter().map(|item| item.default())
    }
}

// All the information about a particular configuration item.
struct ConfigItem {
    var_type: Box<Type>,
    name: Ident,
    default_val: Expr,
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
        let var_type = input.parse()?;
        let _: Token![,] = input.parse()?;
        let name = input.parse()?;
        let _: Token![,] = input.parse()?;
        let default_val = input.parse()?;
        Ok(ConfigItem {
            var_type,
            name,
            default_val,
        })
    }
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
    let members = input.members();
    let defaults = input.defaults();
    let struct_code = quote! {
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

    };
    struct_code.into()
}
