struct SimpleHelloState
{
    val: u32,
}

struct SimpleHelloImpl
{
    state: SimpleHelloState,
}

squad::component_impl!{
    pub impl crate::hello::Hello for SimpleHello
    {
        constructor pub fn new() -> Self;
        method async fn say_hello(&self);
        method async fn input(&mut self, val: u32);
        method async fn output(&self) -> u32;
    }
}

impl SimpleHelloImpl
{
    fn new() -> Self
    {
        return SimpleHelloImpl
        {
            state: SimpleHelloState
            {
                val: 0
            }
        };
    }

    async fn say_hello(&self)
    {
        println!("Hello World!");
    }

    async fn input(&mut self, val: u32)
    {
        println!("Input {}", val);
        self.state.val = val;
    }

    async fn output(&self) -> u32
    {
        println!("Outputting {}", self.state.val);
        return self.state.val;
    }
}