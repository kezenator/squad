mod hello;
mod hello_impl;
mod subscriber;

fn main() {
    tracing::subscriber::set_global_default(subscriber::MySubscriber::new()).unwrap();

    let mut builder = tokio::runtime::Builder::new();
    let runtime = builder.build().unwrap();

    runtime.spawn(async {

        let mut component: squad::Component<dyn hello::Hello> = hello_impl::SimpleHello::new();

        component.say_hello().await;
        component.input(3).await;
        let _ = component.output().await;
        component.input(5).await;
        let _ = component.output().await;
    });

    runtime.shutdown_on_idle();

}
