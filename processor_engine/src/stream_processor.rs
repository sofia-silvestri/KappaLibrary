use std::any::{Any, TypeId};
use std::ffi::c_char;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use std::hash::{Hash,DefaultHasher, Hasher};

#[derive(PartialEq)]
pub enum StreamingState {
    Null,
    Initial,
    Running,
    Stopped,
}

#[derive(Debug)]
pub enum StreamingError {
    Ok,
    InvalidStateTransition,
    InvalidParameter,
    InvalidInput,
    InvalidOutput,
    OutOfRange,
    WrongType,
    PathError,
    CreateError,
    ReadError,
    WriteError,
}

#[derive(Clone)]
pub struct DataHeader {
    key: u64,
    name: String,
    description: String,
    data_type: TypeId,
}

impl DataHeader {
    pub fn new(name: &str, description: &str, type_id: TypeId) -> Self {
        let mut hasher = DefaultHasher::new();
        name.to_string().hash(&mut hasher);
        Self {
            key: hasher.finish(),
            name: name.to_string(),
            description: description.to_string(),
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

pub struct StreamData<T> {
    pub header: DataHeader,
    pub data: Box<dyn Any + Send>,
}

pub trait StreamBlock {
    fn get_input_channel<T: 'static + Send + Any + Clone>(&self, key: &str) -> Result<&Sender<T>, StreamingError>;
    fn connect<T: 'static + Send + Any + Clone>(&mut self, key: &str, sender: Sender<T>) -> Result<(), StreamingError>;
    fn get_parameter_value<T: 'static + Send + PartialOrd + Clone>(&self, key: &str) -> Result<T, StreamingError>;
    fn set_parameter_value<T:'static + Send + PartialOrd + Clone>(&mut self, key: &str, value: T) -> Result<(), StreamingError>;
}

pub trait StreamBlockDyn {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn check_state(&self, state: StreamingState) -> bool;
    fn set_state(&mut self, state: StreamingState);
    fn get_input_list(&self) -> Vec<&str>;
    fn get_output_list(&self) -> Vec<&str>;
    fn get_parameter_list(&self) -> Vec<&str>;
}

pub trait StreamProcessor: StreamBlockDyn {
    fn init(&mut self) -> Result<(), StreamingError >{
        if self.check_state(StreamingState::Running) {
            return Err(StreamingError::InvalidStateTransition)
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