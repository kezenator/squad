use proc_macro2::TokenStream;
use quote::quote_spanned;
use crate::method_tools;

pub fn gen_entire_trait_method(
    trait_ident: syn::Ident,
) -> Result<syn::ImplItemMethod, syn::Error>
{
    let mut stream = TokenStream::new();

    let trait_ident_span = trait_ident.span();

    let trait_literal = syn::LitStr::new(&trait_ident.to_string(), trait_ident.span());

    stream.extend(quote_spanned!(trait_ident_span=>
        pub fn trait_description () -> &'static ::squad::TraitDescription
        {
            use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
            struct MyCallsite;
            static META: Metadata<'static> = {
                ::tracing::metadata! {
                    name: concat!(module_path!(), "::", #trait_literal),
                    target: concat!(#trait_literal),
                    level: ::tracing::Level::DEBUG,
                    fields: ::tracing::fieldset!(),
                    callsite: &MyCallsite,
                    kind: ::tracing_core::Kind::SPAN,
                }
            };
            static INTEREST: AtomicUsize = AtomicUsize::new(0);
            static REGISTRATION: Once = Once::new();
            impl MyCallsite {
                #[inline]
                #[allow(dead_code)]
                fn interest(&self) -> Interest {
                    match INTEREST.load(Ordering::Relaxed) {
                        0 => Interest::never(),
                        2 => Interest::always(),
                        _ => Interest::sometimes(),
                    }
                }
            }
            impl ::tracing_core::callsite::Callsite for MyCallsite {
                fn set_interest(&self, interest: Interest) {
                    let interest = match () {
                        _ if interest.is_never() => 0,
                        _ if interest.is_always() => 2,
                        _ => 1,
                    };
                    INTEREST.store(interest, Ordering::SeqCst);
                }

                fn metadata(&self) -> &Metadata {
                    &META
                }
            }

            static TRAIT_DESCRIPTION: ::squad::TraitDescription = ::squad::TraitDescription {
                module_path: module_path!(),
                trait_name: #trait_literal,
                metadata: &META,
            };

            REGISTRATION.call_once(|| {
                callsite::register(&MyCallsite);
            });

            &TRAIT_DESCRIPTION
        }
    ));

    return Ok(syn::parse(stream.into())?);
}

pub fn gen_trait_method_method(
    trait_ident: syn::Ident,
    method: &syn::TraitItemMethod
) -> Result<syn::ImplItemMethod, syn::Error>
{
    let mut stream = TokenStream::new();

    let method_ident = &method.sig.ident;

    let trait_ident_span = trait_ident.span();

    let trait_literal = syn::LitStr::new(&trait_ident.to_string(), trait_ident.span());
    let method_literal = syn::LitStr::new(&method_ident.to_string(), method_ident.span());

    let non_receiver_argument_idents = method_tools::get_non_receiver_idents(method)?;

    stream.extend(quote_spanned!(trait_ident_span=>
        pub fn #method_ident () -> &'static ::squad::MethodDescription
        {
            use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
            struct MyCallsite;
            static META: Metadata<'static> = {
                ::tracing::metadata! {
                    name: concat!(module_path!(), "::", #trait_literal, "::", #method_literal),
                    target: concat!(#trait_literal, "::", #method_literal),
                    level: ::tracing::Level::DEBUG,
                    fields: ::tracing::fieldset!( #(#non_receiver_argument_idents ,)* return),
                    callsite: &MyCallsite,
                    kind: ::tracing_core::Kind::SPAN,
                }
            };
            static INTEREST: AtomicUsize = AtomicUsize::new(0);
            static REGISTRATION: Once = Once::new();
            impl MyCallsite {
                #[inline]
                #[allow(dead_code)]
                fn interest(&self) -> Interest {
                    match INTEREST.load(Ordering::Relaxed) {
                        0 => Interest::never(),
                        2 => Interest::always(),
                        _ => Interest::sometimes(),
                    }
                }
            }
            impl ::tracing_core::callsite::Callsite for MyCallsite {
                fn set_interest(&self, interest: Interest) {
                    let interest = match () {
                        _ if interest.is_never() => 0,
                        _ if interest.is_always() => 2,
                        _ => 1,
                    };
                    INTEREST.store(interest, Ordering::SeqCst);
                }

                fn metadata(&self) -> &Metadata {
                    &META
                }
            }

            static METHOD_DESCRIPTION: ::squad::MethodDescription = ::squad::MethodDescription {
                module_path: module_path!(),
                trait_name: #trait_literal,
                method_name: #method_literal,
                metadata: &META,
            };

            REGISTRATION.call_once(|| {
                callsite::register(&MyCallsite);
            });

            &METHOD_DESCRIPTION
        }
    ));

    return Ok(syn::parse(stream.into())?);
}
