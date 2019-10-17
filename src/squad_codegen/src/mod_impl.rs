use quote::quote_spanned;
use crate::error::*;
use crate::method_tools;

mod kw
{
    syn::custom_keyword!(constructor);
    syn::custom_keyword!(method);
    syn::custom_keyword!(callback);
}

#[derive(Clone)]
enum MethodKind
{
    Constructor{token: kw::constructor, vis: syn::Visibility},
    Method(kw::method),
    Callback(kw::callback),
}

impl Into<method_tools::MethodCheckKind> for MethodKind
{
    fn into(self) -> method_tools::MethodCheckKind
    {
        match self
        {
            MethodKind::Constructor{token: _, vis: _} => method_tools::MethodCheckKind::Constructor,
            MethodKind::Method(_) => method_tools::MethodCheckKind::Method,
            MethodKind::Callback(_) => method_tools::MethodCheckKind::Callback,
        }
    }
}

impl syn::parse::Parse for MethodKind
{
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error>
    {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::constructor)
        {
            Ok(MethodKind::Constructor{token: input.parse()?, vis: input.parse()?})
        }
        else if lookahead.peek(kw::method)
        {
            Ok(MethodKind::Method(input.parse()?))
        }
        else if lookahead.peek(kw::callback)
        {
            Ok(MethodKind::Callback(input.parse()?))
        }
        else
        {
            Err(lookahead.error())
        }
    }
}

struct Method
{
    kind: MethodKind,
    def: syn::TraitItemMethod,
}

impl syn::parse::Parse for Method
{
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error>
    {
        Ok(Method {
            kind: input.parse()?,
            def: input.parse()?,
        })
    }
}

struct Impl
{
    vis: syn::Visibility,
    #[allow(dead_code)]
    impl_token: syn::Token![impl],
    trait_path: syn::Path,
    #[allow(dead_code)]
    for_token: syn::Token![for],
    ident : syn::Ident,
    #[allow(dead_code)]
    brace_token: syn::token::Brace,
    methods: Vec<Method>,
}

impl syn::parse::Parse for Impl
{
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error>
    {
        let content;

        return Ok(Impl{
            vis: input.parse()?,
            impl_token: input.parse()?,
            trait_path: input.parse()?,
            for_token: input.parse()?,
            ident: input.parse()?,
            brace_token: syn::braced!(content in input),
            methods: {
                let mut items = Vec::new();
                while !content.is_empty() {
                    items.push(content.parse()?);
                }
                items
            },
        });
    }
}

pub fn process_impl(input: proc_macro::TokenStream) -> Result<proc_macro2::TokenStream, syn::Error>
{
    let data: Impl = syn::parse(input)?;

    for method in data.methods.iter()
    {
        method_tools::check_trait_method(&method.def, method.kind.clone().into())?;
    }

    let vis = data.vis.clone();
    let component_ident = data.ident.clone();
    let component_ident_span = component_ident.span();
    let impl_ident = syn::Ident::new(&format!("{}Impl", data.ident.to_string()), data.ident.span());
    let trait_path = data.trait_path.clone();

    let custom_methods = custom_methods(&data)?;
    let trait_methods = trait_methods(&data)?;

    return Ok(quote_spanned!(component_ident_span=>
        #vis struct #component_ident
        {
            value: #impl_ident,
        }

        impl #component_ident
        {
            #custom_methods
        }

        impl #trait_path for #component_ident
        {
            #trait_methods
        }
    ));
}

fn custom_methods(data: &Impl) -> Result<proc_macro2::TokenStream, syn::Error>
{
    let mut stream = proc_macro2::TokenStream::new();

    for method in data.methods.iter()
    {
        match &method.kind
        {
            MethodKind::Constructor{token: _, vis: method_vis} =>
            {
                let impl_ident = &data.ident;
                let method_ident = &method.def.sig.ident;
                let method_ident_span = method_ident.span();
                let trait_path = &data.trait_path;
                let impl_impl_ident = syn::Ident::new(&format!("{}Impl", data.ident.to_string()), data.ident.span());
                let trait_description_path = convert_trait_path(&data.trait_path, "TraitDescription");

                stream.extend(quote_spanned!(method_ident_span=>
                    #method_vis fn #method_ident() -> ::squad::Component<dyn #trait_path>
                    {
                        let impl_ptr = Box::new(
                            #impl_ident
                            { 
                                value: #impl_impl_ident::#method_ident(),
                            });

                        return ::squad::Component::<dyn #trait_path>::new(
                            impl_ptr,
                            #trait_description_path::trait_description());
                    }
                ));
            },
            MethodKind::Method(_) =>
            {
                // These go in the "impl Trait for TraitImpl" section - not here
            },
            MethodKind::Callback(_) =>
            {
                // TODO
                error(method.def.sig.ident.span(), "TODO - callback is not supported yet")?;
            },
        }
    }

    return Ok(stream);
}

fn trait_methods(data: &Impl) -> Result<proc_macro2::TokenStream, syn::Error>
{
    let mut stream = proc_macro2::TokenStream::new();

    for method in data.methods.iter()
    {
        match method.kind
        {
            MethodKind::Constructor{token: _, vis: _} =>
            {
                // These go in the "impl TraitImpl" section - not here
            },
            MethodKind::Method(_) =>
            {
                if method.def.sig.asyncness.is_some()
                {
                    let async_def = method_tools::unwrap_async(&method.def)?.sig;
                    let impl_ident = &data.ident;
                    let method_ident = &method.def.sig.ident;
                    let method_ident_span = method_ident.span();
                    let method_output = &method.def.sig.output;
                    let trait_descriptions_path = convert_trait_path(&data.trait_path, "TraitDescription");
                    let method_descriptions_path = convert_trait_path(&data.trait_path, "MethodDescriptions");

                    let receiver = method_tools::get_receiver(&method.def)?;

                    let self_ident = syn::Ident::new("__self", receiver.self_token.span);
                    let self_mut = &receiver.mutability;

                    let non_receiver_args = method_tools::get_non_receiver_arguments(&method.def)?;
                    let non_receiver_names = method_tools::get_non_receiver_idents(&method.def)?;


                    stream.extend(quote_spanned!(method_ident_span=>
                        #async_def
                        {
                            async fn #method_ident(#self_ident: & #self_mut #impl_ident, #(#non_receiver_args),*) #method_output {

                                let __trait_meta = #trait_descriptions_path :: trait_description().metadata;
                                let __trait_span = ::tracing::Span::child_of(
                                    ::tracing::Span::current(),
                                    __trait_meta,
                                    &::tracing::valueset!(__trait_meta.fields(), ));
                                let __trait_enter = __trait_span.enter();

                                let __trait_method_meta = #method_descriptions_path :: #method_ident().metadata;
                                let __trait_method_span = ::tracing::Span::child_of(
                                    __trait_span.id(),
                                    __trait_method_meta,
                                    &::tracing::valueset!(
                                        __trait_method_meta.fields(),
                                        #(#non_receiver_names = #non_receiver_names),*
                                    ));
                                let __trait_method_enter = __trait_method_span.enter();

                                let __result = #self_ident.value.#method_ident(#(#non_receiver_names),*).await;

                                __trait_method_span.record("return", &::tracing_core::field::debug(&__result));

                                return __result;
                            }

                            Box::pin(#method_ident(self, #(#non_receiver_names),*))
                        }
                    ));
                }
                else // not async
                {
                    error(method.def.sig.ident.span(), "INTERNAL ERROR - non-async trait methods not supported")?;
                }
            },
            MethodKind::Callback(_) =>
            {
                // These go in the "impl TraitImpl" section - not here
            },
        }
    }

    return Ok(stream);
}

fn convert_trait_path(path: &syn::Path, suffix: &str) -> syn::Path
{
    let mut result = path.clone();
    let last = result.segments.pop().unwrap();
    let new = match last
    {
        syn::punctuated::Pair::Punctuated(_, _) => panic!("Internal Error"),
        syn::punctuated::Pair::End(end) =>
            syn::PathSegment{ident: syn::Ident::new(&format!("{}{}", end.ident.to_string(), suffix), end.ident.span()), arguments: syn::PathArguments::None},
    };
    result.segments.push(new);
    return result;
}