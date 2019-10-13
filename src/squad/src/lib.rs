use std::ops::Deref;

pub struct MethodDescription
{
    pub crate_name: &'static str,
    pub module_path: &'static str,
    pub trait_name: &'static str,
    pub method_name: &'static str,
    pub callsite_metadata: &'static ::tracing_core::Metadata<'static>,
}

pub struct Component<T: ?Sized>
{
    value: Box<T>,
    method: &'static MethodDescription,
}

impl<T: ?Sized> Component<T>
{
    pub fn new(value: Box<T>, method: &'static MethodDescription) -> Self
    {
        return Component{value, method};
    }
}

impl<T: ?Sized> Deref for Component<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}