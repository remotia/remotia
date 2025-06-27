extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemStruct, Meta, NestedMeta};

#[proc_macro_attribute]
pub fn buffers_map(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemStruct);
    let args = parse_macro_input!(args as AttributeArgs);

    // Extract the field name from the macro arguments
    let field_name = if let Some(NestedMeta::Meta(Meta::Path(path))) = args.first() {
        path.get_ident().unwrap().clone()
    } else {
        panic!("Expected a single identifier as an argument");
    };

    // Get the name of the struct
    let name = input.ident.clone();

    // Ensure the struct has the specified field
    let fields = if let syn::Fields::Named(fields_named) = input.fields.clone() {
        fields_named.named
    } else {
        panic!("Struct must have named fields");
    };

    let field_exists = fields.iter().any(|field| {
        field
            .ident
            .as_ref()
            .map(|ident| *ident == field_name)
            .unwrap_or(false)
    });

    if !field_exists {
        panic!("Struct must have a field named `{}`", field_name);
    }

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
