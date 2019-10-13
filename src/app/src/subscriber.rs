use tracing::{Event, Id, Metadata, Subscriber};
use tracing::span::{Record, Attributes};

pub struct MySubscriber
{
    counter: std::sync::atomic::AtomicU64,
}

impl MySubscriber
{
    pub fn new() -> Self
    {
        return MySubscriber {
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

impl Subscriber for MySubscriber
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> tracing::subscriber::Interest
    {
        println!("tracing: register_callsite {:?}", metadata);
        println!();
        return tracing::subscriber::Interest::always();
    }

    fn enabled(&self, metadata: &Metadata) -> bool
    {
        println!("tracing: enabled {:?}", metadata);
        println!();
        return true;
    }

    fn new_span(&self, attributes: &Attributes) -> Id
    {
        let next = {
            loop {
                let current = self.counter.load(std::sync::atomic::Ordering::Relaxed);
                if current == self.counter.compare_and_swap(current, current + 1, std::sync::atomic::Ordering::Relaxed)
                {
                    break current + 1;
                }
            }
        };
        println!("tracing: new_span {} {:?}", next, attributes);
        println!();
        return Id::from_u64(next);
    }

    fn record(&self, id: &Id, record: &Record)
    {
        println!("tracing: record {} {:?}", id.into_u64(), record);
        println!();
    }

    fn record_follows_from(&self, _id: &Id, _from: &Id)
    {
        println!("tracing: record_follows_from");
        println!();
    }

    fn event(&self, event: &Event)
    {
        println!("tracing: event {:?}", event);
        println!();
    }

    fn enter(&self, id: &Id)
    {
        println!("tracing: enter {}", id.into_u64());
        println!();
    }

    fn exit(&self, id: &Id)
    {
        println!("tracing: exit {}", id.into_u64());
        println!();
    }

    fn try_close(&self, id: Id) -> bool
    {
        println!("tracing: try_close {}", id.into_u64());
        println!();
        return false;
    }
}

