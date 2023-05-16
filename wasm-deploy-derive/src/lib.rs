use contracts::{generate_impl, get_contracts};
use quote::ToTokens;
use syn::{parse_macro_input, parse_quote, DeriveInput, ItemEnum};

mod contracts;

#[proc_macro_attribute]
pub fn contracts(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let expanded = contract_impl(input).into_token_stream();

    proc_macro::TokenStream::from(expanded)
}

fn contract_impl(input: DeriveInput) -> DeriveInput {
    match input.data {
        syn::Data::Enum(_) => parse_quote! {
            #[derive(
                ::wasm_deploy::clap::Subcommand,
                ::wasm_deploy::derive::Contracts,
                ::wasm_deploy::strum_macros::Display,
                ::wasm_deploy::strum_macros::EnumIter,
                ::wasm_deploy::strum_macros::EnumString,
                ::std::clone::Clone,
                ::std::fmt::Debug,
            )]
            // TODO: figure out how to reexport this attribute macro
            #[clap(rename_all = "snake_case", infer_subcommands = true)]
            #[strum(serialize_all = "snake_case")]
            #input
        },
        _ => panic!("wasm deploy only supports enums"),
    }
}

#[proc_macro_derive(Contracts, attributes(contract))]
pub fn contracts_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ItemEnum);
    let enum_ident = input.ident.clone();
    let contracts = get_contracts(input);

    let impl_expr = generate_impl(&enum_ident, &contracts).into_token_stream();

    proc_macro::TokenStream::from(impl_expr)
}
