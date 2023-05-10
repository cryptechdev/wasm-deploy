use contracts::{generate_enum, generate_impl, generate_use, Contracts};
use proc_macro::{self};
use quote::ToTokens;
use syn::{parse_macro_input, parse_quote, DeriveInput};

mod contracts;

#[proc_macro_attribute]
pub fn contract(
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
                ::wasm_deploy::strum_macros::Display,
                ::wasm_deploy::strum_macros::EnumIter,
                ::wasm_deploy::strum_macros::EnumString,
                ::std::clone::Clone,
                ::std::fmt::Debug,
            )]
            // TODO: figure out how to reexport this attribute macro
            #[strum(serialize_all = "snake_case")]
            #input
        },
        _ => panic!("wasm deploy only supports enums"),
    }
}

///////////////////////////////////////////////////

#[proc_macro]
pub fn contracts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Contracts).0;

    let use_expr = generate_use().into_token_stream();
    let enum_expr = generate_enum(&input).into_token_stream();
    let impl_expr = generate_impl(&input).into_token_stream();

    let total_expr = quote::quote! {
        #use_expr
        #enum_expr
        #impl_expr
    };

    proc_macro::TokenStream::from(total_expr)
}
