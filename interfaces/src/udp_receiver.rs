use std::collections::HashMap;
use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamErrCode, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use data_model::connectors::{ConnectorTrait, Input, Output};

use crate::tcp_receiver::from_bytes;

#[derive(StreamBlockMacro)]
pub struct UdpReceiver<T: 'static + Send + Clone> {
    name:       &'static str,
    inputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters: HashMap<&'static str, Box<dyn DataTrait>>,
    statics:    HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:      HashMap<&'static str, Box<dyn DataTrait>>,
    lock:       Arc<Mutex<()>>,
    proc_state: Arc<Mutex<StreamingState>>,
    phantom:    PhantomData<T>,
    socket:    Option<UdpSocket>,
}
impl<T> UdpReceiver<T> where T: 'static + Send + Clone {
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
            socket: None,
        };
        ret.new_output::<T>("output").unwrap();
        ret.new_statics::<u16>("port", 50000, None).unwrap();
        ret.new_statics::<String>("address", "0.0.0.0".to_string(), None).unwrap();
        ret
    }
}
impl<T> StreamProcessor for UdpReceiver<T> where T: 'static + Send + Clone {
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
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))
            .map_err(|_| StreamErrCode::CreateError)?;
        let dest_addr: Ipv4Addr = address.parse()
            .map_err(|_| StreamErrCode::InvalidStatics)?;
        let interface: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
        if dest_addr.is_multicast() {
            socket.join_multicast_v4(&dest_addr, &interface)
                .map_err(|_| StreamErrCode::CreateError)?;
        }
        self.socket = Some(socket);
        while !self.check_state(StreamingState::Stopped) {
            self.process()?;
        }
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamErrCode> {
        if let Some(socket) = &self.socket {
            let mut buf = vec![0u8; 65536];
            let (amt, _src) = socket.recv_from(&mut buf)
                .map_err(|_| StreamErrCode::ReceiveDataError)?;
            buf.truncate(amt);
            let message = unsafe{from_bytes::<T>(&buf[0..amt])}
                .map_err(|_| StreamErrCode::ReceiveDataError)?;
            self.send_output::<T>("output", message.clone())?;
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
