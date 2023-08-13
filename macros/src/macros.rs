use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use crate::error::AlipayError;

pub fn derive_alipay_param(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    impl_map_macro(&input).unwrap()
}

fn impl_map_macro(input: &DeriveInput) -> Result<TokenStream, AlipayError> {
    let _struct = match &input.data {
        Data::Struct(data) => data,
        Data::Enum(_) => return Err(AlipayError::new("Must be struct type")),
        Data::Union(_) => return Err(AlipayError::new("Must be struct type")),
    };

    let field_name = match &_struct.fields {
        Fields::Named(name) => name,
        Fields::Unnamed(_) => return Err(AlipayError::new("struct must hava field")),
        Fields::Unit => {
            return Err(AlipayError::new(
                "struct type cannot hava punctuation marks",
            ))
        }
    };

    let token_stream: Vec<proc_macro2::TokenStream> = field_name
        .named
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let member = match &field.ident {
                Some(ident) => syn::Member::Named(ident.clone()),
                None => syn::Member::Unnamed(i.into()),
            };

            quote! {
                result.insert(stringify!(#member).to_string(),self.#member.to_alipay_value());
            }
        })
        .collect();

    let _struct_name = &input.ident;
    let (s_impl, s_type, s_where) = input.generics.split_for_impl();
    Ok(quote! {
        impl #s_impl alipay_params::AlipayParams for #_struct_name #s_type #s_where {
            fn to_alipay_value(self) -> alipay_params::AlipayValue {
                let mut result: std::collections::HashMap<String, alipay_params::AlipayValue> = std::collections::HashMap::new();
                #(#token_stream)*
                alipay_params::AlipayValue::from(result)
            }
        }
    }.into())
}
