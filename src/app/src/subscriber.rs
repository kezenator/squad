use tracing::{Event, Id, Metadata, Subscriber};
use tracing::span::{Record, Attributes};
use std::fmt::Write;

std::thread_local!{
    static CUR_SPAN: std::cell::RefCell<Option<u64>> = std::cell::RefCell::new(None);
}

struct StringVisitor<'a>
{
    dest: &'a mut String,
    first: bool,
}

impl<'a> StringVisitor<'a>
{
    fn new(dest: &'a mut String) -> Self
    {
        dest.push('{');
        return StringVisitor{dest, first: true};
    }
}

impl<'a> std::ops::Drop for StringVisitor<'a>
{
    fn drop(&mut self)
    {
        self.dest.push('}');
    }
}

impl<'a> tracing::field::Visit for StringVisitor<'a>
{
    fn record_debug(&mut self, field: &tracing::field::Field, value: & dyn std::fmt::Debug)
    {
        if self.first == true
        {
            self.first = false;
        }
        else
        {
            self.dest.push_str(", ");
        }

        write!(self.dest, "{} = {:?}", field.name(), value).unwrap();
    }
}

struct SpanRecord
{
    ref_count: usize,
    cur_stack: Vec<u64>,
    value: String,
}

struct SharedState
{
    counter: u64,
    map: std::collections::HashMap<u64, SpanRecord>,
}

impl SharedState
{
    fn get_record(&mut self, id: &Id) -> &mut SpanRecord
    {
        let index = id.into_u64();
        return self.map.get_mut(&index).unwrap();
    }
}

pub struct MySubscriber
{
    mutex: std::sync::Mutex<SharedState>,
}

impl MySubscriber
{
    pub fn new() -> Self
    {
        return MySubscriber {
            mutex: std::sync::Mutex::new(
                SharedState{
                    counter: 1,
                    map: std::collections::HashMap::new(),
                })
        };
    }
}

impl Subscriber for MySubscriber
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> tracing::subscriber::Interest
    {
        if *metadata.level() >= tracing::Level::DEBUG
        {
            tracing::subscriber::Interest::always()
        }
        else
        {
            tracing::subscriber::Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata) -> bool
    {
        if *metadata.level() >= tracing::Level::DEBUG
        {
            true
        }
        else
        {
            false
        }
    }

    fn new_span(&self, attributes: &Attributes) -> Id
    {
        let mut state = self.mutex.lock().unwrap();

        let result = state.counter;
        state.counter += 1;

        let mut value = String::new();
        value.push_str(&format!("===== {} =====\n", result));
        value.push_str(&format!("{:?}\n", attributes));
        value.push_str(&format!("{}/{}: {}\n", attributes.metadata().target(), attributes.metadata().name(), attributes.values()));

        state.map.insert(result, SpanRecord{ref_count: 1, cur_stack: Vec::new(), value: value});

        return Id::from_u64(result);
    }

    fn record(&self, id: &Id, traced_record: &Record)
    {
        let mut record_string_val = "Record: ".to_string();
        {
            let mut visitor = StringVisitor::new(&mut record_string_val);
            traced_record.record(&mut visitor);
        }
        record_string_val.push('\n');

        let mut state = self.mutex.lock().unwrap();
        let record = state.get_record(id);
        record.value.push_str(&record_string_val);
    }

    fn record_follows_from(&self, _id: &Id, _from: &Id)
    {
    }

    fn event(&self, event: &Event)
    {
        let mut event_string_val = format!("Event: {}/{}: ", event.metadata().target(), event.metadata().name());
        {
            let mut visitor = StringVisitor::new(&mut event_string_val);
            event.record(&mut visitor);
        }
        event_string_val.push('\n');

        CUR_SPAN.with(|cur| {
            match *cur.borrow()
            {
                Some(index) =>
                {
                    let mut state = self.mutex.lock().unwrap();
                    let record = state.get_record(&Id::from_u64(index));
                    record.value.push_str(&event_string_val);
                },
                None =>
                {
                    println!("===== Event with no parent =====");
                    println!("{}", event_string_val);
                },
            }
        });
    }

    fn enter(&self, id: &Id)
    {
        let index = id.into_u64();

        CUR_SPAN.with(|cur| {
            let mut state = self.mutex.lock().unwrap();
            let mut record = state.get_record(id);
            record.ref_count += 1;
            if let Some(cur_index) = *cur.borrow()
            {
                record.cur_stack.push(cur_index);
            }
            *cur.borrow_mut() = Some(index);
        });
    }

    fn exit(&self, id: &Id)
    {
        let mut state = self.mutex.lock().unwrap();

        let final_ref_count = {
            let mut record = state.get_record(&id);
            record.ref_count -= 1;

            CUR_SPAN.with(|cur| {
                *cur.borrow_mut() = record.cur_stack.pop();
            });

            if record.ref_count == 0
            {
                println!("{}", record.value);
            }

            record.ref_count
        };

        if final_ref_count == 0
        {
            state.map.remove(&id.into_u64());
        }
    }

    fn try_close(&self, id: Id) -> bool
    {
        let mut state = self.mutex.lock().unwrap();

        let final_ref_count = {
            let mut record = state.get_record(&id);
            record.ref_count -= 1;

            if record.ref_count == 0
            {
                println!("{}", record.value);
            }

            record.ref_count
        };

        if final_ref_count == 0
        {
            state.map.remove(&id.into_u64());
        }

        return final_ref_count == 0;
    }

    fn clone_span(&self, id: &Id) -> Id
    {
        let mut state = self.mutex.lock().unwrap();
        let mut record = state.get_record(id);
        record.ref_count += 1;
        return id.clone();
    }
}

