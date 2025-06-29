extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemStruct};

#[proc_macro_attribute]
pub fn buffers_map(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemStruct);

    // Extract the field name from the macro arguments
    let field_name = Ident::new(&attr.to_string(), Span::call_site());

    // Get the name of the struct
    let name = input.ident.clone();

    // Generate the implementation of PullableFrameProperties
    let expanded = quote! {
        #input

        impl remotia::traits::PullableFrameProperties<Buffer, BytesMut> for #name {
            fn push(&mut self, key: Buffer, value: BytesMut) {
                self.#field_name.insert(key, value);
            }

            fn pull(&mut self, key: &Buffer) -> Option<BytesMut> {
                self.#field_name.remove(key)
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
