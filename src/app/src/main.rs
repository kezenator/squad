mod hello;
mod hello_impl;
mod subscriber;

/*use tracing_futures::Instrument;

#[tracing::instrument]
async fn another_func()
{
    tracing::debug!("another debug event");
    println!("another func running in worker");
}*/

fn main() {
    tracing::subscriber::set_global_default(subscriber::MySubscriber::new()).unwrap();

    let mut builder = tokio::runtime::Builder::new();
    let runtime = builder.build().unwrap();

    /*runtime.spawn(async {
        tracing::debug!("my debug event");
        println!("now running on a worker thread");
    }.instrument(tracing::trace_span!("b")));

    runtime.spawn(another_func());*/

    runtime.spawn(async {

        let component: squad::Component<dyn hello::Hello> = hello_impl::HelloImpl::new();

        component.say_hello().await
    });

    runtime.shutdown_on_idle();

}
