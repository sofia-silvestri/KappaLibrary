use std::any::Any;
use std::ffi::c_char;
use std::fmt::Debug;
use std::sync::mpsc::SyncSender;
use std::thread;
use std::time::Duration;

use data_model::connectors::{Input, Output};
use data_model::memory_manager::Statics;
use data_model::memory_manager::Parameter;
use data_model::streaming_data::{StreamErrCode, StreamingState};

use serde::Serialize;

pub trait StreamBlock {
    fn new_input<T: 'static + Send + Clone> (&mut self, key: &'static str) -> Result<(), StreamErrCode>;
    fn new_output<T: 'static + Send + Clone> (&mut self, key: &'static str) -> Result<(), StreamErrCode>;
    fn new_state<T: 'static + Send + Send + Sync + Clone + Serialize + PartialOrd + Debug> (&mut self, key: &'static str, value: T) -> Result<(), StreamErrCode>;
    fn new_parameter<T: 'static + Send + Sync + Clone + Serialize + PartialOrd + Debug> (&mut self, key: &'static str, value: T, limits: Option<[T; 2]>) -> Result<(), StreamErrCode>;
    fn new_statics<T: 'static + Send + Sync + Clone + Serialize + PartialEq + PartialOrd+ Debug> (&mut self, key: &'static str, value: T, limits: Option<[T; 2]>) -> Result<(), StreamErrCode>;
    fn get_input<T: 'static + Send + Clone> (&self, key: &str) -> Result<&Input<T>, StreamErrCode>;
    fn get_output<T: 'static + Send + Clone> (&self, key: &str) -> Result<&Output<T>, StreamErrCode>;
    fn get_parameter<T: 'static + Send + Sync + Clone + Debug> (&self, key: &str) -> Result<&Parameter<T>, StreamErrCode>;
    fn get_statics<T: 'static + Send + Sync + Debug> (&self, key: &str) -> Result<&Statics<T>, StreamErrCode>;
    fn get_input_channel<T: 'static + Send + Any + Clone>(&self, key: &str) -> Result<SyncSender<T>, StreamErrCode>;
    fn connect<T: 'static + Send + Any + Clone>(&mut self, key: &str, sender: SyncSender<T>) -> Result<(), StreamErrCode>;
    fn set_parameter_value<T: 'static + Send + Clone + PartialOrd + Clone + Serialize + Sync+ Debug>(&mut self, key: &str, value: T) -> Result<(), StreamErrCode>;
    fn get_parameter_value<T: 'static + Send + Clone + PartialOrd + Clone + Serialize + Sync+Debug>(&self, key: &str) -> Result<T, StreamErrCode>;
    fn set_statics_value<T: 'static + Send + Clone + Serialize + Sync + Debug + PartialOrd + PartialEq>(&mut self, key: &str, value: T) -> Result<(), StreamErrCode>;
    fn get_statics_value<T: 'static + Send + Clone + Serialize + Sync + Debug + PartialOrd + PartialEq>(&self, key: &str) -> Result<T, StreamErrCode>;
    fn set_state_value<T: 'static + Send + Clone + Serialize + Sync + PartialOrd + PartialEq+Debug>(&mut self, key: &str, value: T) -> Result<(), StreamErrCode>;
    fn get_state_value<T: 'static + Send + Clone + Serialize + Sync + PartialOrd + PartialEq+Debug>(&self, key: &str) -> Result<T, StreamErrCode>;
    fn recv_input<T: 'static + Send+Clone> (&mut self, key: &str) -> Result<T, StreamErrCode>;
    fn send_output<T: 'static +  Send+Clone> (&self, key: &str, value: T) -> Result<(), StreamErrCode>;
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
    fn init(&mut self) -> Result<(), StreamErrCode >{
        if self.check_state(StreamingState::Running) {
            return Err(StreamErrCode::InvalidStateTransition)
        }
        if !self.is_initialized() {
            return Err(StreamErrCode::InvalidStatics)
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn run(&mut self) -> Result<(), StreamErrCode >{
        if self.check_state(StreamingState::Stopped) {
            return Err(StreamErrCode::InvalidStateTransition);
        }
        self.set_state(StreamingState::Running);

        while !self.check_state(StreamingState::Stopped) {
            self.process()?;
        }
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamErrCode >{
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), StreamErrCode > {
        self.set_state(StreamingState::Stopped);
        Ok(())
    }
    fn execute_command(&mut self, command: &str, args: Vec<&str>) -> Result<String, StreamErrCode> {
        Err(StreamErrCode::InvalidOperation)
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
        assert_eq!(res.err(), Some(StreamErrCode::InvalidStatics));
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