use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::mem;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use processor_engine::log;
use processor_engine::logger::{LogLevel, Logger,LogEntry};
use processor_engine::task_monitor::TaskManager;
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamErrCode, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use data_model::connectors::{ConnectorTrait, Input, Output};

use crate::tcp_sender::as_byte;

pub unsafe fn from_bytes<T: 'static>(data: &[u8]) -> Result<&T, StreamErrCode> {
    if TypeId::of::<T>() == TypeId::of::<String>() {
        let string = String::from_utf8_lossy(data).into_owned();
        return Ok(unsafe {&*(Box::leak(Box::new(string)) as *mut String as *mut T)});
    }
    if TypeId::of::<T>() == TypeId::of::<str>() {
        let string = String::from_utf8_lossy(data).into_owned();
        let boxed_string: Box<String> = Box::new(string);
        let static_string_ref: &'static mut String = Box::leak(boxed_string);
        let typed_ptr: *mut T = static_string_ref as *mut String as *mut T;
        return Ok(unsafe {&*typed_ptr});
    }
    if data.len() != mem::size_of::<T>() {
        eprintln!("Wrong slice dimension!");
        return Err(StreamErrCode::InvalidInput);
    }
    let ptr = data.as_ptr();
    let ptr_t: *const T = ptr as *const T;
    unsafe {Ok(&*ptr_t)}
}

#[derive(Clone)]
pub struct TcpMessage<T> {
    pub id_stream: u32,
    pub message: T,
}

pub struct TcpHandler<T> where T: 'static + Send + Clone {
    pub stream_id: u32,
    pub stream: TcpStream,
    pub data_sender: Output<TcpMessage<T>>,
    pub receiver: Receiver<TcpMessage<T>>,
    pub sender: SyncSender<TcpMessage<T>>,
}

impl<T> TcpHandler<T> where T: 'static + Send + Clone {
    pub fn new(stream_id: u32,
                  stream: TcpStream,
                  data_sender: Output<TcpMessage<T>>) -> Self 
    where T: 'static + Send + Clone
    {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<TcpMessage<T>>(100);
        Self {
            stream_id,
            stream,
            data_sender,
            receiver,
            sender,
        }
    }
    pub fn get_sender(&self) -> SyncSender<TcpMessage<T>> 
    where T: 'static + Send + Clone
    {
        self.sender.clone()
    }
    pub fn handle_stream(&mut self) -> Result<(), String> 
    {
        let mut buffer = [0; 65535];
        match self.stream.read(&mut buffer) {
            Ok(n) => {
                let data = unsafe{from_bytes::<T>(&buffer[0..n])};
                match data {
                    Ok(data) => {
                        let message = TcpMessage {
                            id_stream: self.stream_id as u32,
                            message: data.clone(),
                        };
                        let _ = self.data_sender.send(message);
                        let recv_message = self.receiver.recv();
                        match recv_message {
                            Ok(msg) => {
                                if self.stream.write_all(as_byte::<T>(&msg.message)).is_err() {
                                    return Err("Server: write stream error".to_string());
                                }   
                                Ok(())
                            }
                            Err(_) => Err("Handler: receive message error".to_string()),
                        }
                    }
                    Err(_) => {
                        if self.stream.write_all("Invalid format\n".as_bytes()).is_err() {
                            return Err("Server: write stream error".to_string());
                        }
                        Err(("Invalid data format received").to_string())
                    }
                }
            }
            Err(e) => {
                Err(format!("Server: read stream error: {}", e))
            }
        }
    }
}

unsafe impl<T> Sync for TcpHandler<T> where T: 'static + Send + Clone {}

#[derive(StreamBlockMacro)]
pub struct TcpReceiver<T: 'static + Send + Clone> {
    name:       &'static str,
    inputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters: HashMap<&'static str, Box<dyn DataTrait>>,
    statics:    HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:      HashMap<&'static str, Box<dyn DataTrait>>,
    lock:       Arc<Mutex<()>>,
    proc_state: Arc<Mutex<StreamingState>>,
    pub logger: Logger,
    tcp_listen: Option<TcpListener>,
    tcp_stream: HashMap<u32, Arc<Mutex<TcpHandler<T>>>>,
    tcp_handle: Vec<JoinHandle<()>>,
}

impl<T> TcpReceiver<T> 
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
            logger: Logger::new(Some(name)),
            tcp_listen: None,
            tcp_stream: HashMap::new(),
            tcp_handle: Vec::new(),
        };
        ret.new_input::<TcpMessage<T>>("response").unwrap();
        ret.new_output::<TcpMessage<T>>("received").unwrap();
        ret.new_statics::<u16>("port", 50000, None).unwrap();
        ret.new_statics::<String>("address", "0.0.0.0".to_string(), None).unwrap();
        ret
    }
    pub fn receiver_loop(handler: Arc<Mutex<TcpHandler<T>>>, logger_input: SyncSender<LogEntry>, name: &'static str) {
        loop {
            let exit = THREAD_EXIT.get().unwrap().lock().unwrap();
            if *exit {
                break;
            }
            match handler.lock().unwrap().handle_stream() {
                Ok(_) => {}
                Err(e) => {
                    let log_entry = LogEntry::new(
                        LogLevel::Error, 
                        name.to_string(),
                         e.clone());
                    logger_input.send(log_entry).unwrap();
                    break;
                }
            }
        }
    }
    pub fn send_answer(&self, message: TcpMessage<T>) -> Result<(), StreamErrCode> {

        if let Some(handler) = self.tcp_stream.get(&message.id_stream) {
            let sender = handler.lock().unwrap().get_sender();
            sender.send(message).map_err(|_| StreamErrCode::SendDataError)
        } else {
            Err(StreamErrCode::InvalidInput)
        }
    }
        
}

impl<T> StreamProcessor for TcpReceiver<T> 
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
        match TcpListener::bind(format!("{}:{}", address, port)) {
            Ok(tcp_listen) => {self.tcp_listen = Some(tcp_listen);}
            Err(_) => {
                self.set_state(StreamingState::Stopped);
                return Err(StreamErrCode::SendDataError);
            }
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn run(&mut self) -> Result<(), StreamErrCode> {
        self.set_state(StreamingState::Running);
        if self.tcp_listen.is_none() {
            self.set_state(StreamingState::Stopped);
            return Err(StreamErrCode::SendDataError);
        }
        self.process()?;
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamErrCode > {
        let mut counter_stream: u32 = 0;
        for stream in self.tcp_listen.as_ref().unwrap().incoming() {
            if stream.is_ok() {
                counter_stream += 1;
                log!(self.logger, LogLevel::Info, self.name, "New connection.");
                let _lock = self.lock.lock().unwrap();
                let output = self.get_output::<TcpMessage<T>>("received").expect("").clone();
                let mut tm = TaskManager::get().lock().unwrap();
                let tcp_handler = TcpHandler::new(counter_stream, stream.as_ref().unwrap().try_clone().unwrap(), output.clone());
                let tcp_handler_arc = Arc::new(Mutex::new(tcp_handler));
                self.tcp_stream.insert(counter_stream, tcp_handler_arc.clone());
                let name = self.name;
                let logger_input = self.logger.get_input("LogEntry").unwrap().sender.clone();
                let handle = tm.create_task(name, move || {
                    Self::receiver_loop(tcp_handler_arc, logger_input, name);
                });
                match handle {
                    Ok(handle) => {
                        self.tcp_handle.push(handle);
                    }
                    Err(e) => {}
                }
            }
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), StreamErrCode > {
        THREAD_EXIT.get_or_init(|| Arc::new(Mutex::new(true)));
        for j in self.tcp_handle.drain(..) {
            let _ = j.join();
        }
        self.set_state(StreamingState::Stopped);
        Ok(())
    }
}

static THREAD_EXIT: OnceLock<Arc<Mutex<bool>>> = OnceLock::new();