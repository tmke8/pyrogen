use cache_key::derive_cache_key;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod cache_key;
mod config;

#[proc_macro_derive(ConfigurationOptions, attributes(option, doc, option_group))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    config::derive_impl(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates a [`CacheKey`] implementation for the attributed type.
///
/// Struct fields can be attributed with the `cache_key` field-attribute that supports:
/// * `ignore`: Ignore the attributed field in the cache key
#[proc_macro_derive(CacheKey, attributes(cache_key))]
pub fn cache_key(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);

    let result = derive_cache_key(&item);
    let stream = result.unwrap_or_else(|err| err.to_compile_error());

    TokenStream::from(stream)
}
