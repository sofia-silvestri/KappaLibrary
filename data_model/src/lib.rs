pub mod modules;
pub mod streaming_error;

use std::any::Any;

use crate::streaming_error::StreamingError;


pub trait StaticsTrait {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> Option<&mut dyn Any>;
    fn is_settable(&self) -> bool;
}

#[derive(Copy, Clone)]
pub struct Statics<T: 'static> {
    value: T,
    settable: bool,
}
impl<T: 'static> StaticsTrait for Statics<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        if self.settable {
            Some(self)
        } else {
            None
        }
    }
    fn is_settable(&self) -> bool {
        self.settable
    }
}
impl<T: 'static> Statics<T> {
    pub fn new(value: T) -> Self {
        Self { value, settable: true }
    }
    pub fn set(&mut self, value: T) -> Result<(), StreamingError> {
        if self.settable {
            self.value = value;
            self.settable = false;
            Ok(())
        } else {
            Err(StreamingError::InvalidOperation)
        }
    }
    pub fn get(&self) -> &T {
        &self.value
    }
}

