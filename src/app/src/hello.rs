#[squad_codegen::component]
pub trait Hello
{
    async fn say_hello(&self);
}
