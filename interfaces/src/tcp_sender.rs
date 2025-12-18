use std::collections::HashMap;
use std::any::Any;
use std::fmt::Debug;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::mem;
use std::net::TcpStream;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamErrCode, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use data_model::connectors::{ConnectorTrait, Input, Output};

pub fn as_byte<T>(value: &T) -> &[u8] {
    let ptr = value as *const T;
    let byte_ptr: *const u8 = ptr as *const u8;
    
    unsafe {
        std::slice::from_raw_parts(
            byte_ptr, 
            mem::size_of::<T>()
        )
    }
}

#[derive(StreamBlockMacro)]
pub struct TcpSender<T: 'static + Send + Clone> {
    name:       &'static str,
    inputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters: HashMap<&'static str, Box<dyn DataTrait>>,
    statics:    HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:      HashMap<&'static str, Box<dyn DataTrait>>,
    lock:       Arc<Mutex<()>>,
    proc_state: Arc<Mutex<StreamingState>>,
    phantom:    PhantomData<T>,
    tcp_stream: Option<TcpStream>,
}

impl<T> TcpSender<T> 
where 
    T: 'static + Send + Clone
{
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
            phantom: PhantomData,
            tcp_stream: None,
        };
        ret.new_input::<T>("input").unwrap();
        ret.new_statics::<u16>("port", 50000, None).unwrap();
        ret.new_statics::<String>("address", "0.0.0.0".to_string(), None).unwrap();
        ret
    }
}

impl<T> StreamProcessor for TcpSender<T> 
where T: 'static + Send + Clone
{
    fn init(&mut self) -> Result<(), StreamErrCode > {
        if self.check_state(StreamingState::Running) {
            self.set_state(StreamingState::Stopped);
            return  Err(StreamErrCode::InvalidStateTransition);
        }
        if !self.is_initialized() {
            return  Err(StreamErrCode::InvalidStatics);
        }
        let port = self.get_statics_value::<u16>("port").expect("");
        let address = self.get_statics_value::<String>("address").expect("");
        match TcpStream::connect(format!("{}:{}", address, port)) {
            Ok(tcp_stream) => {self.tcp_stream = Some(tcp_stream);}
            Err(_) => {
                self.set_state(StreamingState::Stopped);
                return Err(StreamErrCode::SendDataError);
            }
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamErrCode> {
        if self.tcp_stream.is_none() {
            self.set_state(StreamingState::Stopped);
            return Err(StreamErrCode::SendDataError);
        }
        match self.recv_input::<T>("input") {
            Ok(input) => {
                let error_send: bool;
                let stream = self.tcp_stream.as_mut().unwrap();
                {               
                    let _lock = self.lock.lock().unwrap();
                    match stream.write(as_byte::<T>(&input)) {
                        Ok(_) => {
                            let mut buffer = [0; 65535];
                            match stream.read(&mut buffer) {
                                Ok(n) => {
                                    let response = String::from_utf8_lossy(&buffer[0..n]);
                                    if response != "Ok" {
                                        error_send = true;
                                    } else {error_send = false;}
                                }
                                Err(_) => {error_send = true;}
                            }
                        }
                        Err(_) => {error_send = true;}
                    };
                }                
                if error_send {
                    self.set_state(StreamingState::Stopped);
                    return Err(StreamErrCode::SendDataError);
                }
            }
            Err(e) => {return Err(e);}
        }
        Ok(())
    }
}