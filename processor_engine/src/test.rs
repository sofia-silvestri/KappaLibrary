use std::collections::HashMap;
use std::any::Any;
use std::fmt::Display;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamingError, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use crate::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use crate::connectors::{ConnectorTrait, Input, Output};


#[derive(StreamBlockMacro)]
pub struct TestBlock {
    name:       &'static str,
    inputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters: HashMap<&'static str, Box<dyn DataTrait>>,
    statics:    HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:      HashMap<&'static str, Box<dyn DataTrait>>,
    lock:       Arc<Mutex<()>>,
    proc_state: Arc<Mutex<StreamingState>>,
}
impl TestBlock {
    pub fn new(name: &'static str) -> Self {
        let mut ret = Self {
            name,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            parameters: HashMap::new(),
            statics: HashMap::new(),
            state: HashMap::new(),
            lock: Arc::new(Mutex::new(())),
            proc_state: Arc::new(Mutex::new(StreamingState::Null)),
        };
        ret.new_input::<i32>("test_input");
        ret.new_output::<f32>("test_output");
        ret.new_parameter::<bool>("change_sign", false, None);
        ret.new_statics::<i32>("sum_value", 0);

        ret
    }
}
impl StreamProcessor for TestBlock {
    fn process(&mut self) -> Result<(), StreamingError >{
        let change_sign = self.get_parameter_value::<bool>("change_sign").unwrap();
        let sum_value = self.get_statics::<i32>("sum_value").unwrap().get_value();
        let value = self.recv_input::<i32>("test_input").unwrap();
        let out_value: f32;
        if !change_sign {
            out_value = (value + sum_value) as f32;
        } else {
            out_value = -(value + sum_value) as f32;
        }
        self.send_output::<f32>("test_output", out_value)

    }
}