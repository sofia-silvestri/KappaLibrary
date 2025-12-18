use std::collections::HashMap;
use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::UdpSocket;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamErrCode, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use data_model::connectors::{ConnectorTrait, Input, Output};

use crate::tcp_sender::as_byte;

#[derive(StreamBlockMacro)]
pub struct UdpSender<T: 'static + Send + Clone> {
    name:       &'static str,
    inputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters: HashMap<&'static str, Box<dyn DataTrait>>,
    statics:    HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:      HashMap<&'static str, Box<dyn DataTrait>>,
    lock:       Arc<Mutex<()>>,
    proc_state: Arc<Mutex<StreamingState>>,
    socket:    Option<UdpSocket>,
    phantom:    PhantomData<T>,
}
impl<T> UdpSender<T> where T: 'static + Send + Clone {
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
            socket: None,
            phantom: PhantomData,
        };
        ret.new_input::<T>("input").unwrap();
        ret.new_statics::<u16>("port", 50000, None).unwrap();
        ret.new_statics::<String>("address", "0.0.0.0".to_string(), None).unwrap();
        ret
    }
}
impl<T> StreamProcessor for UdpSender<T> where T: 'static + Send + Clone {
    fn init(&mut self) -> Result<(), StreamErrCode> {
        if self.check_state(StreamingState::Running) {
            return Err(StreamErrCode::InvalidStateTransition)
        }
        if !self.is_initialized() {
            return Err(StreamErrCode::InvalidStatics)
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn run(&mut self) -> Result<(), StreamErrCode> {
        if self.check_state(StreamingState::Stopped) {
            return Err(StreamErrCode::InvalidStateTransition);
        }
        self.set_state(StreamingState::Running);
        let port = self.get_statics_value::<u16>("port")?;
        let address = self.get_statics_value::<String>("address")?;
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|_| StreamErrCode::CreateError)?;
        socket.connect(format!("{}:{}", address, port))
            .map_err(|_| StreamErrCode::CreateError)?;
        self.socket = Some(socket);
        while self.check_state(StreamingState::Running) {
            self.process()?;
        }
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamErrCode> {
        let input = self.recv_input::<T>("input")?;
        if let Some(socket) = &self.socket {
            let bytes = as_byte::<T>(&input);
            if socket.send(&bytes).map_err(|_| StreamErrCode::SendDataError).is_err() {
                return Err(StreamErrCode::SendDataError);
            }
            Ok(())
        } else {
            Err(StreamErrCode::FileNotFound)
        }
    }
    fn stop(&mut self) -> Result<(), StreamErrCode> {
        self.set_state(StreamingState::Stopped);
        Ok(())
    }
}
