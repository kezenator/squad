use tracing::{Event, Id, Metadata, Subscriber};
use tracing_core::span::Current;
use tracing::span::{Record, Attributes};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ValueOutput
{
    name: &'static str,
    value: String,
}

pub struct SpanOutput
{
    pub id: u64,
    pub metadata: &'static tracing::Metadata<'static>,
    pub values: Vec<ValueOutput>,
    pub contents: Vec<ActionOutput>,
}

#[derive(Clone)]
pub struct EventOutput
{
    pub metadata: &'static tracing::Metadata<'static>,
    pub values: Vec<ValueOutput>,
}

#[derive(Clone)]
pub struct RecordOutput
{
    pub values: Vec<ValueOutput>,
}

#[derive(Clone)]
pub enum ActionOutput
{
    Span(Arc<Mutex<SpanOutput>>),
    Record(RecordOutput),
    Event(EventOutput),
}

pub enum RootOutput
{
    Span(Arc<Mutex<SpanOutput>>),
    Event(EventOutput),
}

std::thread_local!{
    static CUR_SPAN: RefCell<Option<u64>> = RefCell::new(None);
}

struct VecValueOutputVisitor
{
    result: Option<Vec<ValueOutput>>,
}

impl VecValueOutputVisitor
{
    fn new() -> Self
    {
        VecValueOutputVisitor{ result: Some(Vec::new()) }
    }

    fn take(&mut self) -> Vec<ValueOutput>
    {
        self.result.take().unwrap()
    }
}

impl tracing::field::Visit for VecValueOutputVisitor
{
    fn record_debug(&mut self, field: &tracing::field::Field, value: & dyn std::fmt::Debug)
    {
        if let Some(result) = &mut self.result
        {
            result.push(
                ValueOutput{
                    name: field.name(),
                    value: format!("{:?}", value),
                });
        }
    }
}

struct OpenSpanDetails
{
    ref_count: usize,
    is_root: bool,
    cur_stack: Vec<u64>,
    output: Arc<Mutex<SpanOutput>>,
}

struct SharedState
{
    counter: u64,
    map: std::collections::HashMap<u64, OpenSpanDetails>,
}

impl SharedState
{
    fn get_record(&mut self, id: &Id) -> &mut OpenSpanDetails
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

    fn output(&self, output: RootOutput)
    {
        let mut prefix = String::new();
        match output
        {
            RootOutput::Event(event) => self.output_prefixed(&mut prefix, ActionOutput::Event(event)),
            RootOutput::Span(span) => self.output_prefixed(&mut prefix, ActionOutput::Span(span)),
        }
    }

    fn output_prefixed(&self, prefix: &mut String, action: ActionOutput)
    {
        match action
        {
            ActionOutput::Span(span_arc_mutex) =>
            {
                let span = span_arc_mutex.lock().unwrap();
                println!("{}===== {} =====", prefix, span.id);
                prefix.push_str(" | ");
                self.output_prefixed(prefix, ActionOutput::Record(RecordOutput{ values: span.values.clone()}));
                for sub_action in span.contents.iter()
                {
                    self.output_prefixed(prefix, sub_action.clone());
                }
                prefix.pop();
                prefix.pop();
                prefix.pop();
            },
            ActionOutput::Event(event) =>
            {
                println!("{}Event: {:?}", prefix, event.values);
            },
            ActionOutput::Record(record) =>
            {
                println!("{}Record: {:?}", prefix, record.values);
            },
        }
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

        // Create a new ID for this span

        let result = state.counter;
        state.counter += 1;

        // Collect up the values

        let mut visitor = VecValueOutputVisitor::new();
        attributes.record(&mut visitor);

        // Create a new SpanOutput for the results

        let span_output = Arc::new(Mutex::new(SpanOutput
            {
                id: result,
                metadata: attributes.metadata(),
                values: visitor.take(),
                contents: Vec::new(),
            }));

        // If there's a parent, then add this
        // span to it's set of actions

        if let Some(parent_id) = attributes.parent()
        {
            let parent_record = state.get_record(&parent_id);
            parent_record.output.lock().unwrap().contents.push(ActionOutput::Span(span_output.clone()));
        }

        // Finally, insert this span into the
        // map of active spans

        state.map.insert(
            result,
            OpenSpanDetails{
                ref_count: 1,
                is_root: attributes.parent().is_none(),
                cur_stack: Vec::new(),
                output: span_output,
            });

        return Id::from_u64(result);
    }

    fn record(&self, id: &Id, traced_record: &Record)
    {
        let mut visitor = VecValueOutputVisitor::new();
        traced_record.record(&mut visitor);

        let mut state = self.mutex.lock().unwrap();
        let record = state.get_record(id);
        record.output.lock().unwrap().contents.push(ActionOutput::Record(RecordOutput{values: visitor.take()}));
    }

    fn record_follows_from(&self, _id: &Id, _from: &Id)
    {
    }

    fn event(&self, event: &Event)
    {
        let values = {
            let mut visitor = VecValueOutputVisitor::new();
            event.record(&mut visitor);
            visitor.take()
        };

        CUR_SPAN.with(|cur| {
            match *cur.borrow()
            {
                Some(index) =>
                {
                    let mut state = self.mutex.lock().unwrap();
                    let record = state.get_record(&Id::from_u64(index));
                    record.output.lock().unwrap().contents.push(ActionOutput::Event(EventOutput{metadata: event.metadata(), values}));
                },
                None =>
                {
                    self.output(RootOutput::Event(EventOutput{metadata: event.metadata(), values}));
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

            record.ref_count
        };

        if final_ref_count == 0
        {
            if let Some(open_span_details) = state.map.remove(&id.into_u64())
            {
                if open_span_details.is_root
                {
                    self.output(RootOutput::Span(open_span_details.output));
                }
            }
        }
    }

    fn try_close(&self, id: Id) -> bool
    {
        let mut state = self.mutex.lock().unwrap();

        let final_ref_count = {
            let mut record = state.get_record(&id);
            record.ref_count -= 1;

            record.ref_count
        };

        if final_ref_count == 0
        {
            if let Some(open_span_details) = state.map.remove(&id.into_u64())
            {
                if open_span_details.is_root
                {
                    self.output(RootOutput::Span(open_span_details.output));
                }
            }
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

    fn current_span(&self) -> Current
    {
        let mut state = self.mutex.lock().unwrap();
        CUR_SPAN.with(|cur| {
            match *cur.borrow()
            {
                Some(index) =>
                {
                    let id = Id::from_u64(index);
                    let record = state.get_record(&id);
                    Current::new(id, record.output.lock().unwrap().metadata)
                },
                None => Current::none(),
            }
        })
    }
}
