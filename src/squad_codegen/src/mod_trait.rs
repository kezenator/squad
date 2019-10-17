use proc_macro2::*;
use crate::error::*;
use syn::parse;
use quote::*;
use crate::method_tools;

pub fn process_trait(input: proc_macro::TokenStream) -> Result<proc_macro2::TokenStream, syn::Error>
{
    let mut item : syn::ItemTrait = syn::parse(input)?;

    check_trait(&item)?;

    let extras = modify(&mut item)?;

    return Ok(quote!(#item #extras));
}

pub fn modify(item: &mut syn::ItemTrait) -> Result<TokenStream, syn::Error>
{
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
                crate::method_tools::unwrap_async_inplace(&mut *method)?;
            },
            _ => {},
        }
    }

    return generate_extras(item);
}

fn generate_extras(item: &syn::ItemTrait) -> Result<TokenStream, syn::Error>
{
    let ident_span = item.ident.span();
    let type_descriptions_name = quote::format_ident!("{}TraitDescription", item.ident);
    let type_descriptions_method = crate::gen_traits::gen_entire_trait_method(item.ident.clone())?;

    let type_descriptions_struct : syn::ItemStruct = parse(quote_spanned!(ident_span=>
        pub struct #type_descriptions_name
        {
        }
    ).into())?;

    let type_descriptions_impl : syn::ItemImpl = parse(quote_spanned!(ident_span=>
        impl #type_descriptions_name
        {
            #type_descriptions_method
        }
    ).into())?;

    let mut method_descriptions_vec : Vec<syn::ImplItemMethod> = Vec::new();

    for member in item.items.iter()
    {
        match member
        {
            syn::TraitItem::Method(method) =>
            {
                method_descriptions_vec.push(
                    crate::gen_traits::gen_trait_method_method(
                        item.ident.clone(),
                        &method,
                    )?
                );
            },
            _ => {},
        }
    }

    let method_descriptions_name = quote::format_ident!("{}MethodDescriptions", item.ident);
    let method_descriptions_struct : syn::ItemStruct = parse(quote_spanned!(ident_span=>
        pub struct #method_descriptions_name
        {
        }
    ).into())?;

    let method_descriptions_impl : syn::ItemImpl = parse(quote_spanned!(ident_span=>
        impl #method_descriptions_name
        {
            #(#method_descriptions_vec)*
        }
    ).into())?;

    return Ok(quote!(
        #type_descriptions_struct
        #type_descriptions_impl
        #method_descriptions_struct
        #method_descriptions_impl
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
                let _ = crate::method_tools::check_trait_method(method, method_tools::MethodCheckKind::Method)?;
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