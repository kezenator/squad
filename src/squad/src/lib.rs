use std::ops::{Deref, DerefMut};

pub use squad_codegen::component_trait;
pub use squad_codegen::component_impl;

pub struct TraitDescription
{
    pub module_path: &'static str,
    pub trait_name: &'static str,
    pub metadata: &'static ::tracing_core::Metadata<'static>,
}

pub struct MethodDescription
{
    pub module_path: &'static str,
    pub trait_name: &'static str,
    pub method_name: &'static str,
    pub metadata: &'static ::tracing_core::Metadata<'static>,
}

pub struct Component<T: ?Sized>
{
    value: Box<T>,
}

impl<T: ?Sized> Component<T>
{
    pub fn new(
        value: Box<T>,
        _trait_description: &'static TraitDescription) -> Self
    {
        return Component{value};
    }
}

impl<T: ?Sized> Deref for Component<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> DerefMut for Component<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
