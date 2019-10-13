use proc_macro2::*;
use crate::error::*;
use syn::parse;
use quote::quote;

pub fn modify(item: &mut syn::ItemTrait) -> Result<TokenStream, syn::Error>
{
    // Check that it all looks OK before we start making changes

    check_trait(item)?;

    // Add the Sync and Send super traits

    item.supertraits.push(parse(quote!(::std::marker::Sync).into())?);
    item.supertraits.push(parse(quote!(::std::marker::Send).into())?);

    // Change all of the methods

    for member in item.items.iter_mut()
    {
        match member
        {
            syn::TraitItem::Method(method) =>
            {
                if method.sig.asyncness.is_some()
                {
                    // If it's marked as async, we need to convert
                    // it to non-async, by:
                    // 1) Removing the async keywoard
                    // 2) Adding a lifetime generic
                    // 3) Adding the lifetime to the receiver
                    // 4) Changing the return type to a future

                    method.sig.asyncness = None;

                    method.sig.generics.params.push(parse(quote!('s).into())?);

                    for param in method.sig.inputs.iter_mut()
                    {
                        if let syn::FnArg::Receiver(receiver) = param
                        {
                            receiver.reference = Some(
                                (parse(quote!(&).into())?,
                                Some(parse(quote!('s).into())?)));
                        }
                    }

                    let output : syn::ReturnType = match &method.sig.output
                    {
                        syn::ReturnType::Default =>
                        {
                            parse(quote!(-> ::std::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = ()> + Send + 's>>).into())?
                        },
                        syn::ReturnType::Type(rarrow, ret_type) =>
                        {
                            parse(quote!(#rarrow ::std::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = #ret_type > + Send + 's>>).into())?
                        },
                    };

                    method.sig.output = output;
                }
            },
            _ => {},
        }
    }

    return generate_extras(item);
}

fn generate_extras(item: &syn::ItemTrait) -> Result<TokenStream, syn::Error>
{
    let type_traits_name = quote::format_ident!("{}TypeTraits", item.ident);
    let type_traits_method = crate::gen_traits::gen_entire_trait_method(item.ident.clone())?;

    let type_traits_struct : syn::ItemStruct = parse(quote!(
        pub struct #type_traits_name
        {
        }
    ).into())?;

    let type_traits_impl : syn::ItemImpl = parse(quote!(
        impl #type_traits_name
        {
            #type_traits_method
        }
    ).into())?;

    let mut method_traits_vec : Vec<syn::ImplItemMethod> = Vec::new();

    for member in item.items.iter()
    {
        match member
        {
            syn::TraitItem::Method(method) =>
            {
                method_traits_vec.push(
                    crate::gen_traits::gen_trait_method_method(
                        item.ident.clone(),
                        method.sig.ident.clone(),
                    )?
                );
            },
            _ => {},
        }
    }

   let method_traits_name = quote::format_ident!("{}MethodTraits", item.ident);
    let method_traits_struct : syn::ItemStruct = parse(quote!(
        pub struct #method_traits_name
        {
        }
    ).into())?;
    let method_traits_impl : syn::ItemImpl = parse(quote!(
        impl #method_traits_name
        {
            #(#method_traits_vec)*
        }
    ).into())?;

    return Ok(quote!(
        #type_traits_struct
        #type_traits_impl
        #method_traits_struct
        #method_traits_impl
    ));
}

fn check_trait(item: &syn::ItemTrait) -> Result<(), syn::Error>
{
    // Check that it's public, not unsafe and has no generics

    if let syn::Visibility::Public(_) = item.vis
    {
        // Ok
    }
    else
    {
        error(item.ident.span(), "component trait must be public")?;
    }

    if let Some(syn::token::Unsafe{span: _}) = item.unsafety
    {
        error(item.ident.span(), "component trait must be public")?;
    }

    if !item.supertraits.is_empty()
    {
        error(item.ident.span(), "component trait must have no super traits")?;
    }

    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        error(item.ident.span(), "component trait must have no type or lifetime parameters")?;
    }

    // Check that there are only methods,
    // and that the methods meet the requirements

    for member in item.items.iter()
    {
        match member
        {
            syn::TraitItem::Const(const_item) =>
            {
                error(const_item.ident.span(), "component trait must only have methods")?;
            },
            syn::TraitItem::Method(method) =>
            {
                // Check it has no attributes and no body,
                // not unsafe, ...

                if !method.attrs.is_empty()
                {
                    error(method.sig.ident.span(), "component trait methods must have no attributes")?;
                }

                if method.default.is_some()
                {
                    error(method.sig.ident.span(), "component trait methods must have no default implementation")?;
                }

                if method.sig.constness.is_some()   
                {
                    error(method.sig.ident.span(), "component trait methods must not be const")?;
                }

                if method.sig.unsafety.is_some()
                {
                    error(method.sig.ident.span(), "component trait methods must not be unsafe")?;
                }

                if method.sig.abi.is_some()
                {
                    error(method.sig.ident.span(), "component trait methods must not have an ABI")?;
                }

                // Check there are no generics

                if !method.sig.generics.params.is_empty()
                    || method.sig.generics.where_clause.is_some()
                {
                    error(method.sig.ident.span(), "component trait methods can not have type or lifetime parameters")?;
                }

                // Check there is a receiver, it's by reference, and
                // all parameters are by value

                let mut got_receiver = false;

                for input in method.sig.inputs.iter()
                {
                    match input
                    {
                        syn::FnArg::Receiver(receiver) =>
                        {
                            if got_receiver
                            {
                                error(receiver.self_token.span, "component trait method has two receivers")?;
                            }

                            got_receiver = true;

                            if !receiver.attrs.is_empty()
                            {
                                error(method.sig.ident.span(), "component trait method cannot have attributes")?;
                            }

                            match &receiver.reference
                            {
                                Some((_, opt_lifetime)) =>
                                {
                                    if opt_lifetime.is_some()
                                    {
                                        error(method.sig.ident.span(), "component trait method receiver must not have a lifetime")?;
                                    }
                                },
                                None =>
                                {
                                    error(method.sig.ident.span(), "component trait method receiver must be by reference")?;
                                },
                            }
                        },
                        syn::FnArg::Typed(pat) =>
                        {
                            if !pat.attrs.is_empty()
                            {
                                error(method.sig.ident.span(), "component trait method cannot have attributes")?;
                            }
                        },
                    }
                }

                if !got_receiver
                {
                    error(method.sig.ident.span(), "component trait method must have a receiver")?;
                }
            },
            syn::TraitItem::Type(type_item) =>
            {
                error(type_item.ident.span(), "component trait must only have methods")?;
            },
            syn::TraitItem::Macro(macro_item) =>
            {
                error(macro_item.mac.bang_token.spans[0], "component trait must only have methods")?;
            },
            _ =>
            {
                error(item.ident.span(), "component trait must only have methods")?;
            },
        }
    }

    Ok(())
}