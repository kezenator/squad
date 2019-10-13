extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod error;
mod gen_traits;
mod mod_trait;

use quote::quote;

#[proc_macro_attribute]
pub fn component(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let result = do_it(input);

    let proc2_token_stream = match result
    {
        Ok(proc2_token_stream) => proc2_token_stream,
        Err(syn_error) => syn_error.to_compile_error(),
    };

    let result_stream = proc_macro::TokenStream::from(proc2_token_stream);

    println!();
    println!("{}", result_stream.to_string());
    println!();

    return result_stream;
}

fn do_it(input: proc_macro::TokenStream) -> Result<proc_macro2::TokenStream, syn::Error>
{
    let mut item : syn::ItemTrait = syn::parse(input)?;

    let extras = mod_trait::modify(&mut item)?;

    return Ok(quote!(#item #extras));
}
