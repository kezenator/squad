use crate::error::*;
use quote::quote_spanned;
use syn::parse;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum MethodCheckKind
{
    Constructor,
    Method,
    Callback,
}

pub fn check_trait_method(method: &syn::TraitItemMethod, kind: MethodCheckKind) -> Result<(), syn::Error>
{
    // Check it has no attributes and no body,
    // not unsafe, ...

    if !method.attrs.is_empty()
    {
        method_error(method.sig.ident.span(), kind, "must have no attributes")?;
    }

    if method.default.is_some()
    {
        method_error(method.sig.ident.span(), kind, "must have no default implementation")?;
    }

    if method.sig.constness.is_some()   
    {
        method_error(method.sig.ident.span(), kind, "must not be const")?;
    }

    if method.sig.unsafety.is_some()
    {
        method_error(method.sig.ident.span(), kind, "must not be unsafe")?;
    }

    if method.sig.abi.is_some()
    {
        method_error(method.sig.ident.span(), kind, "must not have an ABI")?;
    }

    // Check there are no generics

    if !method.sig.generics.params.is_empty()
        || method.sig.generics.where_clause.is_some()
    {
        method_error(method.sig.ident.span(), kind, "cannot have type or lifetime parameters")?;
    }

    // Check for async-ness

    match kind
    {
        MethodCheckKind::Constructor =>
        {
            if method.sig.asyncness.is_some()
            {
                method_error(method.sig.ident.span(), kind, "cannot be async")?;
            }
        }
        MethodCheckKind::Method
            | MethodCheckKind::Callback =>
        {
            if method.sig.asyncness.is_none()
            {
                method_error(method.sig.ident.span(), kind, "must be async")?;
            }
        }
    }

    // Check all of the parameters

    let mut got_receiver = false;

    for input in method.sig.inputs.iter()
    {
        match input
        {
            syn::FnArg::Receiver(receiver) =>
            {
                if got_receiver
                {
                    method_error(receiver.self_token.span, kind, "cannot multiple receivers")?;
                }

                if kind == MethodCheckKind::Constructor
                {
                    method_error(receiver.self_token.span, kind, "cannot have a receiver")?;
                }

                got_receiver = true;

                if !receiver.attrs.is_empty()
                {
                    method_error(method.sig.ident.span(), kind, "cannot have attributes")?;
                }

                match &receiver.reference
                {
                    Some((_, opt_lifetime)) =>
                    {
                        if opt_lifetime.is_some()
                        {
                            method_error(method.sig.ident.span(), kind, "must not have a lifetime")?;
                        }
                    },
                    None =>
                    {
                        method_error(method.sig.ident.span(), kind, "must be by reference")?;
                    },
                }
            },
            syn::FnArg::Typed(pat) =>
            {
                if !pat.attrs.is_empty()
                {
                    method_error(method.sig.ident.span(), kind, "cannot have attributes")?;
                }
            },
        }
    }

    if !got_receiver
        && kind != MethodCheckKind::Constructor
    {
        method_error(method.sig.ident.span(), kind, "must have a receiver")?;
    }

    Ok(())
}

fn method_error(span: proc_macro2::Span, kind: MethodCheckKind, msg: &str) -> Result<(), syn::Error>
{
    let prefix = match kind
    {
        MethodCheckKind::Constructor => "component constructor ",
        MethodCheckKind::Method => "component method ",
        MethodCheckKind::Callback => "component callback ",
    };

    return error(span, &format!("{}{}", prefix, msg));
}

pub fn unwrap_async_inplace(method: &mut syn::TraitItemMethod) -> Result<(), syn::Error>
{
    if method.sig.asyncness.is_some()
    {
        // If it's marked as async, we need to convert
        // it to non-async, by:
        // 1) Removing the async keywoard
        // 2) Adding a lifetime generic
        // 3) Adding the lifetime to the receiver
        // 4) Changing the return type to a future

        let ident_span = method.sig.ident.span();

        method.sig.asyncness = None;

        method.sig.generics.params.push(parse(quote_spanned!(ident_span=> 's).into())?);

        for param in method.sig.inputs.iter_mut()
        {
            if let syn::FnArg::Receiver(receiver) = param
            {
                receiver.reference = Some(
                    (parse(quote_spanned!(ident_span=> &).into())?,
                    Some(parse(quote_spanned!(ident_span=> 's).into())?)));
            }
        }

        let output : syn::ReturnType = match &method.sig.output
        {
            syn::ReturnType::Default =>
            {
                parse(quote_spanned!(ident_span=> -> ::std::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = ()> + Send + 's>>).into())?
            },
            syn::ReturnType::Type(rarrow, ret_type) =>
            {
                parse(quote_spanned!(ident_span=> #rarrow ::std::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = #ret_type > + Send + 's>>).into())?
            },
        };

        method.sig.output = output;
    }

    return Ok(());
}

pub fn unwrap_async(method: &syn::TraitItemMethod) -> Result<syn::TraitItemMethod, syn::Error>
{
    let mut result = method.clone();
    unwrap_async_inplace(&mut result)?;
    return Ok(result);
}

pub fn get_receiver(method: &syn::TraitItemMethod) -> Result<syn::Receiver, syn::Error>
{
    for input in method.sig.inputs.iter()
    {
        match input
        {
            syn::FnArg::Receiver(receiver) =>
            {
                return Ok(receiver.clone());
            },
            _ => {},
        }
    }

    return Err(error_val(method.sig.ident.span(), "INTERNAL ERROR: method has no receiver"));
}

pub fn get_non_receiver_arguments(method: &syn::TraitItemMethod) -> Result<Vec<syn::PatType>, syn::Error>
{
    let mut result = Vec::new();

    for input in method.sig.inputs.iter()
    {
        match input
        {
            syn::FnArg::Receiver(_) => {},
            syn::FnArg::Typed(typed) =>
            {
                result.push(typed.clone());
            },
        }
    }

    return Ok(result);
}

pub fn get_non_receiver_idents(method: &syn::TraitItemMethod) -> Result<Vec<syn::Ident>, syn::Error>
{
    let mut result = Vec::new();

    for arg in get_non_receiver_arguments(method)?
    {
        match &*arg.pat
        {
            syn::Pat::Ident(pat_ident) =>
            {
                result.push(pat_ident.ident.clone());
            },
            _ =>
            {
                error(arg.colon_token.spans[0], "Unsupported trait method parameter")?;
            }
        }
    }

    return Ok(result);
}