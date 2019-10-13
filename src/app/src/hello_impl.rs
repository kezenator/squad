pub struct HelloImpl
{
}

impl HelloImpl
{
    pub fn new() -> ::squad::Component<dyn crate::hello::Hello>
    {
        struct InternalImpl
        {
            value: HelloImpl,
        }

        impl InternalImpl
        {
            fn new() -> Self
            {
                return InternalImpl{ value: HelloImpl{} };
            }
        }

        impl crate::hello::Hello for InternalImpl
        {
            fn say_hello<'s>(&'s self) -> std::pin::Pin<Box<dyn core::future::Future<Output = ()> + Send + 's>>
            {
                async fn say_hello(_self: &InternalImpl) {

                    let meta = crate::hello::HelloMethodTraits::say_hello().callsite_metadata;
                    let span = tracing::Span::child_of(
                        tracing::Span::current(),
                        meta,
                        &::tracing::valueset!(meta.fields(), ));
                    let _enter = span.enter();

                    _self.value.say_hello().await
                }

                Box::pin(say_hello(self))
            }
        }

        return ::squad::Component::<dyn crate::hello::Hello>::new(
            Box::new(InternalImpl::new()),
            crate::hello::HelloMethodTraits::say_hello()
        );
    }
}

impl HelloImpl
{
    async fn say_hello(&self)
    {
        println!("Hello World!");
    }
}