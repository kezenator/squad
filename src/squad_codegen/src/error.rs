use proc_macro2::Span;
use core::fmt::Display;

pub fn error<T: Display>(span: Span, message: T) -> Result<(), syn::Error> {
    return Err(syn::Error::new(span, message));
}

pub fn error_val<T: Display>(span: Span, message: T) -> syn::Error {
    return syn::Error::new(span, message);
}
