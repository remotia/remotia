extern crate proc_macro;

use core::panic;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Ident, ItemStruct};

#[proc_macro_attribute]
pub fn buffers_map(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemStruct);

    // Extract the field name from the macro arguments
    let field_name = Ident::new(&attr.to_string(), Span::call_site());

    // Get the name of the struct
    let name = input.ident.clone();

    // Find the field in the struct by its name
    let field = input
        .fields
        .iter()
        .find(|f| f.ident.as_ref() == Some(&field_name))
        .unwrap();

    // Extract the type of the field
    let field_type_tokens = &field
        .ty
        .to_token_stream()
        .into_iter()
        .collect::<Vec<TokenTree>>();

    let ty = field_type_tokens
        .get(0)
        .expect("Unable to read field first token");

    if ty.to_string() != "BuffersMap" {
        panic!("The field should be of type BuffersMap");
    }

    let key_type_name = Ident::new(&field_type_tokens
        .get(2)
        .expect("Unable to read key type")
        .to_string(), Span::call_site());

    // Generate the implementation of PullableFrameProperties
    let expanded = quote! {
        #input

        impl remotia::traits::PullableFrameProperties<#key_type_name, BytesMut> for #name {
            fn push(&mut self, key: #key_type_name, value: BytesMut) {
                self.#field_name.insert(key, value);
            }

            fn pull(&mut self, key: &#key_type_name) -> Option<BytesMut> {
                self.#field_name.remove(key)
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
