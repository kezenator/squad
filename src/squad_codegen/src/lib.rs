extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod error;
mod gen_traits;
mod method_tools;
mod mod_trait;
mod mod_impl;

#[proc_macro_attribute]
pub fn component_trait(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    return run(mod_trait::process_trait, input)
}

#[proc_macro]
pub fn component_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    return run(mod_impl::process_impl, input)
}

fn run<F: FnOnce(proc_macro::TokenStream) -> Result<proc_macro2::TokenStream, syn::Error>>(process: F, input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let result = process(input);

    let proc2_token_stream = match result
    {
        Ok(proc2_token_stream) => proc2_token_stream,
        Err(syn_error) => syn_error.to_compile_error(),
    };

    let result_stream = proc_macro::TokenStream::from(proc2_token_stream);

    //println!();
    //println!("{}", result_stream.to_string());
    //println!();

    return result_stream;
}
