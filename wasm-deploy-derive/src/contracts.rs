use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
};

use convert_case::{Case, Casing};
use quote::{
    ToTokens,
    __private::{Span, TokenStream},
};

use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    token::Brace,
    Expr, ExprMatch, Ident, ItemEnum, ItemImpl, LitStr, Path, Token,
};
pub fn generate_match<F>(enum_ident: &Ident, contracts: &[Contract], f: F) -> ExprMatch
where
    F: Fn(&Contract) -> Expr,
{
    let match_statement_base = ExprMatch {
        attrs: vec![],
        match_token: parse_quote!(match),
        expr: parse_quote!(&self),
        brace_token: Brace::default(),
        arms: contracts
            .iter()
            .map(|contract| {
                let ident = Ident::new(
                    contract.name.to_case(Case::UpperCamel).as_str(),
                    Span::call_site(),
                );
                let expr = f(contract);

                parse_quote!(
                   #enum_ident::#ident => {
                       #expr
                   }
                )
            })
            .collect(),
    };

    parse_quote! {
        #match_statement_base
    }
}

pub fn get_contracts(item_enum: ItemEnum) -> Vec<Contract> {
    item_enum
        .variants
        .into_iter()
        .map(|variant| {
            let attr = variant
                .attrs
                .into_iter()
                .find(|attr| attr.path().is_ident("contract"))
                .expect("Missing `#[contract(..)]` attribute");

            let options: Options = attr.parse_args().unwrap();

            let name = variant.ident.to_string();

            Contract {
                name,
                admin: options.admin,
                instantiate: options.instantiate,
                execute: options.execute,
                query: options.query,
                migrate: options.migrate,
                cw20_send: options.cw20_send,
            }
        })
        .collect()
}

pub fn generate_impl(enum_ident: &Ident, contracts: &[Contract]) -> ItemImpl {
    let admin_match = generate_match(enum_ident, contracts, |contract| {
        let path = &contract.admin;
        parse_quote!(#path.to_string())
    });

    let instantiate_match = generate_match(enum_ident, contracts, |contract| {
        let path = &contract.instantiate;
        parse_quote!(Ok(Box::new(<#path as ::interactive_parse::InteractiveParseObj>::parse_to_obj()?)))
    });

    let execute_match = generate_match(enum_ident, contracts, |contract| match &contract.execute {
        Some(path) => {
            parse_quote!(Ok(Box::new(<#path as ::interactive_parse::InteractiveParseObj>::parse_to_obj()?)))
        }
        None => {
            parse_quote!(::anyhow::bail!(
                "The ExecuteMsg has not yet been implemented"
            ))
        }
    });

    let query_match = generate_match(enum_ident, contracts, |contract| match &contract.query {
        Some(path) => {
            parse_quote!(Ok(Box::new(<#path as ::interactive_parse::InteractiveParseObj>::parse_to_obj()?)))
        }
        None => {
            parse_quote!(::anyhow::bail!("The QueryMsg has not yet been implemented"))
        }
    });

    let migrate_match = generate_match(enum_ident, contracts, |contract| match &contract.migrate {
        Some(path) => {
            parse_quote!(Ok(Box::new(<#path as ::interactive_parse::InteractiveParseObj>::parse_to_obj()?)))
        }
        None => {
            parse_quote!(::anyhow::bail!(
                "The MigrateMsg has not yet been implemented"
            ))
        }
    });

    let cw20_send_match = generate_match(enum_ident, contracts, |contract| {
        match &contract.cw20_send {
            Some(path) => {
                parse_quote!(Ok(Box::new(<#path as ::interactive_parse::InteractiveParseObj>::parse_to_obj()?)))
            }
            None => {
                parse_quote!(::anyhow::bail!(
                    "The Cw20 Receive message has not yet been implemented"
                ))
            }
        }
    });

    parse_quote! {
        impl ::wasm_deploy::contract::ContractInteractive for #enum_ident {
            fn admin(&self) -> String {
                #admin_match
            }
            fn instantiate(&self) -> Result<Box<dyn ::wasm_deploy::contract::Msg>, ::anyhow::Error> {
                #instantiate_match
            }
            fn execute(&self) -> Result<Box<dyn ::wasm_deploy::contract::Msg>, ::anyhow::Error> {
                #execute_match
            }
            fn query(&self) -> Result<Box<dyn ::wasm_deploy::contract::Msg>, ::anyhow::Error> {
                #query_match
            }
            fn migrate(&self) -> Result<Box<dyn ::wasm_deploy::contract::Msg>, ::anyhow::Error> {
                #migrate_match
            }
            fn cw20_send(&self) -> Result<Box<dyn ::wasm_deploy::contract::Msg>, ::anyhow::Error> {
                #cw20_send_match
            }
        }
    }
}

enum Value {
    Type(syn::Path),
    Str(syn::LitStr),
}

impl Value {
    fn unwrap_type(self) -> syn::Path {
        if let Self::Type(p) = self {
            p
        } else {
            panic!("expected a type");
        }
    }

    fn unwrap_str(self) -> syn::LitStr {
        if let Self::Str(s) = self {
            s
        } else {
            panic!("expected a string literal");
        }
    }
}

// impl Display for Value {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         match self {
//             Self::Type(p) => p.get_ident().unwrap().fmt(f),
//             Self::Str(s) => s.value().fmt(f),
//         }
//     }
// }

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Type(p) => p.to_tokens(tokens),
            Self::Str(s) => s.to_tokens(tokens),
        }
    }
}

impl Parse for Value {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        if let Ok(p) = input.parse::<syn::Path>() {
            Ok(Self::Type(p))
        } else {
            Ok(Self::Str(input.parse::<syn::LitStr>()?))
        }
    }
}

struct Pair((Ident, Value));

impl Parse for Pair {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let k = input.parse::<syn::Ident>()?;
        input.parse::<Token![=]>()?;
        let v = input.parse::<Value>()?;

        Ok(Self((k, v)))
    }
}

pub struct Contract {
    name: String,
    admin: Value,
    instantiate: Path,
    execute: Option<Path>,
    query: Option<Path>,
    migrate: Option<Path>,
    cw20_send: Option<Path>,
}

pub struct Options {
    name: Option<LitStr>,
    admin: Value,
    instantiate: Path,
    execute: Option<Path>,
    query: Option<Path>,
    migrate: Option<Path>,
    cw20_send: Option<Path>,
}

impl Parse for Options {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let pairs = input.parse_terminated(Pair::parse, Token![,])?;
        let mut map: BTreeMap<_, _> = pairs.into_iter().map(|p| p.0).collect();

        let name = map.remove(&parse_quote!(name)).map(|x| x.unwrap_str());

        let admin = map.remove(&parse_quote!(admin)).unwrap();

        let instantiate = map
            .remove(&parse_quote!(instantiate))
            .unwrap()
            .unwrap_type();

        let execute = map
            .remove(&parse_quote!(execute))
            .map(|ty| ty.unwrap_type());

        let query = map.remove(&parse_quote!(query)).map(|ty| ty.unwrap_type());

        let migrate = map
            .remove(&parse_quote!(migrate))
            .map(|ty| ty.unwrap_type());

        let cw20_send = map
            .remove(&parse_quote!(cw20_send))
            .map(|ty| ty.unwrap_type());

        let instantiate_msg = map
            .remove(&parse_quote!(instantiate_msg))
            .map(|ty| ty.to_token_stream());

        if let Some((invalid_option, _)) = map.into_iter().next() {
            panic!("unknown generate_api option: {}", invalid_option);
        }

        Ok(Self {
            name,
            admin,
            instantiate,
            execute,
            query,
            migrate,
            cw20_send,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    #[should_panic(expected = "unknown generate_api option: asd")]
    fn invalid_option() {
        let _options: Options = parse_quote! {
            instantiate: InstantiateMsg,
            asd: Asd,
        };
    }
}
