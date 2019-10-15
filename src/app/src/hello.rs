#[squad::component_trait]
pub trait Hello
{
    async fn say_hello(&self);
    async fn input(&mut self, val: u32);
    async fn output(&self) -> u32;
}
