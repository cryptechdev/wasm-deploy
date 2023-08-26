use std::collections::BTreeMap;

use convert_case::{Case, Casing};

use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    token::Brace,
    Expr, ExprMatch, Ident, ItemEnum, ItemImpl, Path, Token,
};

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

            let name = if let Some(rename) = options.rename {
                rename
            } else {
                let string = variant
                    .ident
                    .to_string()
                    .from_case(Case::UpperCamel)
                    .to_case(Case::Kebab);
                parse_quote!(#string)
            };

            Contract {
                name,
                bin_name: options.bin_name,
                variant_name: variant.ident,
                path: options.path,
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
                let variant_ident = contract.variant_name.clone();
                let expr = f(contract);

                parse_quote!(
                   #enum_ident::#variant_ident => {
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

pub fn generate_impl(enum_ident: &Ident, contracts: &[Contract]) -> ItemImpl {
    let name_match = generate_match(enum_ident, contracts, |contract| {
        let path = &contract.name;
        parse_quote!(#path.to_string())
    });

    let bin_name_match =
        generate_match(enum_ident, contracts, |contract| match &contract.bin_name {
            Some(bin_name) => parse_quote!(#bin_name.to_string()),
            None => parse_quote!(self.name()),
        });

    let path_match = generate_match(enum_ident, contracts, |contract| match &contract.path {
        Some(path) => parse_quote!(#path.into()),
        None => parse_quote!(::std::path::PathBuf::from(format!(
            "contracts/{}",
            self.name()
        ))),
    });

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
            fn name(&self) -> String {
                #name_match
            }
            fn bin_name(&self) -> String {
                #bin_name_match
            }
            fn path(&self) -> ::std::path::PathBuf {
                #path_match
            }
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
    Path(syn::Path),
    Expr(syn::Expr),
}

impl Value {
    fn unwrap_type(self) -> syn::Path {
        if let Self::Path(p) = self {
            p
        } else {
            panic!("expected a type");
        }
    }

    fn unwrap_expr(self) -> syn::Expr {
        if let Self::Expr(e) = self {
            e
        } else {
            panic!("expected an expression");
        }
    }
}

struct Pair((Ident, Value));

impl Parse for Pair {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let k = input.parse::<syn::Ident>()?;
        input.parse::<Token![=]>()?;
        let v = match k.to_string().as_str() {
            // "rename" => Value::Str(input.parse::<LitStr>()?),
            "admin" | "rename" | "bin_name" | "path" => Value::Expr(input.parse::<Expr>()?),
            "instantiate" | "execute" | "query" | "migrate" | "cw20_send" => {
                Value::Path(input.parse::<Path>()?)
            }
            _ => return Err(syn::Error::new(
                k.span(),
                "expected one of: rename, admin, instantiate, execute, query, migrate, cw20_send",
            )),
        };

        Ok(Self((k, v)))
    }
}

pub struct Contract {
    name: Expr,
    bin_name: Option<Expr>,
    path: Option<Expr>,
    variant_name: Ident,
    admin: Expr,
    instantiate: Path,
    execute: Option<Path>,
    query: Option<Path>,
    migrate: Option<Path>,
    cw20_send: Option<Path>,
}

pub struct Options {
    rename: Option<Expr>,
    bin_name: Option<Expr>,
    path: Option<Expr>,
    admin: Expr,
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

        let rename = map.remove(&parse_quote!(rename)).map(|x| x.unwrap_expr());

        let bin_name = map.remove(&parse_quote!(bin_name)).map(|x| x.unwrap_expr());

        let path = map.remove(&parse_quote!(path)).map(|x| x.unwrap_expr());

        let admin = map.remove(&parse_quote!(admin)).unwrap().unwrap_expr();

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

        if let Some((invalid_option, _)) = map.into_iter().next() {
            panic!("unknown generate_api option: {}", invalid_option);
        }

        Ok(Self {
            rename,
            bin_name,
            path,
            admin,
            instantiate,
            execute,
            query,
            migrate,
            cw20_send,
        })
    }
}
