use proc_macro2::Span;
use quote::ToTokens;
use core::fmt::Display;

pub fn error<T: Display>(span: Span, message: T) -> Result<(), syn::Error> {
    return Err(syn::Error::new(span, message));
}

pub fn error_spanned<T: ToTokens, U: Display>(tokens: T, message: U) -> Result<(), syn::Error> {
    return Err(syn::Error::new_spanned(tokens, message));
}
