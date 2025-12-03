use std::any::Any;
use std::ffi::c_char;
use std::fmt::Display;
use std::sync::mpsc::SyncSender;
use std::thread;
use std::time::Duration;

use crate::connectors::{Input, Output};
use data_model::memory_manager::Statics;
use data_model::memory_manager::Parameter;
use data_model::streaming_data::{StreamingError, StreamingState};

use serde::Serialize;

pub trait StreamBlock {
    fn new_input<T: 'static + Send + Clone> (&mut self, key: &'static str) -> Result<(), StreamingError>;
    fn new_output<T: 'static + Send + Clone> (&mut self, key: &'static str) -> Result<(), StreamingError>;
    fn new_state<T: 'static + Send + Send + Sync + Clone + Serialize + PartialOrd + Display> (&mut self, key: &'static str, value: T) -> Result<(), StreamingError>;
    fn new_parameter<T: 'static + Send + Sync + Clone + Serialize + PartialOrd + Display> (&mut self, key: &'static str, value: T, limits: Option<[T; 2]>) -> Result<(), StreamingError>;
    fn new_statics<T: 'static + Send + Sync + Clone + Serialize + PartialEq + PartialOrd+ Display> (&mut self, key: &'static str, value: T, limits: Option<[T; 2]>) -> Result<(), StreamingError>;
    fn get_input<T: 'static + Send + Clone> (&self, key: &str) -> Result<&Input<T>, StreamingError>;
    fn get_output<T: 'static + Send + Clone> (&self, key: &str) -> Result<&Output<T>, StreamingError>;
    fn get_parameter<T: 'static + Send + Sync + Clone + Display> (&self, key: &str) -> Result<&Parameter<T>, StreamingError>;
    fn get_statics<T: 'static + Send + Sync + Display> (&self, key: &str) -> Result<&Statics<T>, StreamingError>;
    fn get_input_channel<T: 'static + Send + Any + Clone>(&self, key: &str) -> Result<SyncSender<T>, StreamingError>;
    fn connect<T: 'static + Send + Any + Clone>(&mut self, key: &str, sender: SyncSender<T>) -> Result<(), StreamingError>;
    fn get_parameter_value<T: 'static + Send + Clone + PartialOrd + Clone + Serialize + Sync+Display>(&self, key: &str) -> Result<T, StreamingError>;
    fn set_parameter_value<T: 'static + Send + Clone + PartialOrd + Clone + Serialize + Sync+ Display>(&mut self, key: &str, value: T) -> Result<(), StreamingError>;
    fn set_statics_value<T: 'static + Send + Clone + Serialize + Sync + Display + PartialOrd + PartialEq>(&mut self, key: &str, value: T) -> Result<(), StreamingError>;
    fn get_state_value<T: 'static + Send + Clone + Serialize + Sync + PartialOrd + PartialEq+Display>(&mut self, key: &str) -> Result<T, StreamingError>;
    fn set_state_value<T: 'static + Send + Clone + Serialize + Sync + PartialOrd + PartialEq+Display>(&mut self, key: &str, value: T) -> Result<(), StreamingError>;
    fn recv_input<T: 'static + Send+Clone> (&mut self, key: &str) -> Result<T, StreamingError>;
    fn send_output<T: 'static +  Send+Clone> (&self, key: &str, value: T) -> Result<(), StreamingError>;
}

pub trait StreamBlockDyn : Send {
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
        self.set_state(StreamingState::Stopped);
        Ok(())
    }
    fn execute_command(&mut self, command: &str, args: Vec<&str>) -> Result<String, StreamingError> {
        Err(StreamingError::InvalidOperation)
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::TestBlock;

    #[test]
    fn test_processor() {
        let mut test_block = TestBlock::new("test");
        let res = test_block.init();
        assert!(res.is_err());
        assert_eq!(res.err(), Some(StreamingError::InvalidStatics));
        let res = test_block.set_statics_value("sum_value", 5);
        assert!(res.is_ok());
        assert!(test_block.init().is_ok());
        let sender = test_block.get_input::<i32>("test_input").unwrap().sender.clone();
        let (out_sender, out_receiver) = std::sync::mpsc::sync_channel::<f32>(50);
        test_block.connect("test_output", out_sender);
        
        sender.send(0);
        test_block.process();
        assert_eq!(out_receiver.recv().unwrap(), 5.0);
        sender.send(1);
        test_block.process();
        assert_eq!(out_receiver.recv().unwrap(), 6.0);
        test_block.set_parameter_value("change_sign", true);
        sender.send(1);
        test_block.process();
        assert_eq!(out_receiver.recv().unwrap(), -6.0);
    }
}