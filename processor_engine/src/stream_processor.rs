use std::any::{Any, TypeId};
use std::ffi::c_char;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use std::hash::{Hash,DefaultHasher, Hasher};

use crate::connectors::{Input, Output, Parameter};
use data_model::memory_manager::Statics;
use data_model::streaming_error::StreamingError;
use serde::Serialize;

#[derive(PartialEq)]
pub enum StreamingState {
    Null,
    Initial,
    Running,
    Stopped,
}

#[derive(Clone)]
pub struct DataHeader {
    pub key: u64,
    pub name: &'static str,
    pub description: &'static str,
    data_type: TypeId,
}

impl DataHeader {
    pub fn new(name: &'static str, description: &'static str, type_id: TypeId) -> Self {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self {
            key: hasher.finish(),
            name,
            description,
            data_type: type_id,
        }
    }
    pub fn get_key(&self) -> u64 {
        self.key
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_description(&self) -> &str {
        &self.description
    }
    pub fn get_data_type(&self) -> TypeId {
        self.data_type
    }
}


pub trait StreamBlock {
    fn new_input<T: 'static + Send> (&mut self, key: &'static str, description: &'static str) -> Result<(), StreamingError>;
    fn new_output<T: 'static + Send + Clone> (&mut self, key: &'static str, description: &'static str) -> Result<(), StreamingError>;
    fn new_parameter<T: 'static + Send + Sync + Copy + Clone + Serialize + PartialOrd> (&mut self, key: &'static str, description: &'static str, value: T) -> Result<(), StreamingError>;
    fn new_statics<T: 'static + Send + Sync + Copy + Serialize> (&mut self, key: &'static str, description: &'static str, value: T) -> Result<(), StreamingError>;
    fn get_input<T: 'static + Send> (&self, key: &str) -> Result<&Input<T>, StreamingError>;
    fn get_output<T: 'static + Send> (&self, key: &str) -> Result<&Output<T>, StreamingError>;
    fn get_parameter<T: Send + Sync + Copy + Clone + Serialize> (&self, key: &str) -> Result<&Parameter<T>, StreamingError>;
    fn get_statics<T> (&self, key: &str) -> Result<&Statics<T>, StreamingError>;
    fn get_input_channel<T: 'static + Send + Any + Clone>(&self, key: &str) -> Result<&Sender<T>, StreamingError>;
    fn connect<T: 'static + Send + Any + Clone>(&mut self, key: &str, sender: Sender<T>) -> Result<(), StreamingError>;
    fn get_parameter_value<T: 'static + Send + PartialOrd + Clone + Copy + Serialize + Sync>(&self, key: &str) -> Result<T, StreamingError>;
    fn set_parameter_value<T: 'static + Send + PartialOrd + Clone + Copy + Serialize + Sync>(&mut self, key: &str, value: T) -> Result<(), StreamingError>;
    fn set_statics_value<T: 'static + Send + Clone + Copy + Serialize + Sync>(&mut self, key: &str, value: T) -> Result<(), StreamingError>;
}

pub trait StreamBlockDyn {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn check_state(&self, state: StreamingState) -> bool;
    fn set_state(&mut self, state: StreamingState);
    fn get_input_list(&self) -> Vec<&str>;
    fn get_output_list(&self) -> Vec<&str>;
    fn get_parameter_list(&self) -> Vec<&str>;
    fn get_statics_list(&self) -> Vec<&str>;
    fn is_initialized(&self) -> bool;
    fn get_qualified_name(&self, name: &str) -> &'static str;
}

pub trait StreamProcessor: StreamBlockDyn {
    fn init(&mut self) -> Result<(), StreamingError >{
        if self.check_state(StreamingState::Running) {
            return Err(StreamingError::InvalidStateTransition)
        }
        if !self.is_initialized() {
            return Err(StreamingError::InvalidStatics)
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn run(&mut self) -> Result<(), StreamingError >{
        if self.check_state(StreamingState::Stopped) {
            return Err(StreamingError::InvalidStateTransition);
        }
        self.set_state(StreamingState::Running);

        while !self.check_state(StreamingState::Stopped) {
            self.process()?;
        }
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamingError >{
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), StreamingError > {
        if self.check_state(StreamingState::Initial ){
            return Err(StreamingError::InvalidStateTransition);
        }
        self.set_state(StreamingState::Stopped);
        Ok(())
    }
}

#[repr(C)]
pub struct StreamProcessorStruct {
    pub name: *const c_char,
    pub description: *const c_char,
    pub input_number: u32,
    pub inputs: &'static [*const c_char],
    pub inputs_type: &'static [*const c_char],
    pub output_number: u32,
    pub outputs: &'static [*const c_char],
    pub outputs_type: &'static [*const c_char],
    pub parameter_number: u32,
    pub parameters: &'static [*const c_char],
    pub parameters_type: &'static [*const c_char],
}
unsafe impl Sync for StreamProcessorStruct {}