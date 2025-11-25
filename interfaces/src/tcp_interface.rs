use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::thread::JoinHandle;
use std::fmt::{Display, Debug};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex, OnceLock};
use processor_engine::log;
use processor_engine::logger::{LogLevel, Logger};
use processor_engine::task_monitor::TaskManager;
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamingError, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use processor_engine::connectors::{ConnectorTrait, Input, Output};
use processor_engine::logger::LogEntry;
use std::net::{TcpListener, TcpStream};
use std::mem;


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
pub unsafe fn from_bytes<T: 'static>(data: &[u8]) -> Result<&T, StreamingError> {
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
        return Err(StreamingError::InvalidInput);
    }
    let ptr = data.as_ptr();
    let ptr_t: *const T = ptr as *const T;
    unsafe {Ok(&*ptr_t)}
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
        ret.new_statics::<u16>("port", 50000).unwrap();
        ret.new_statics::<String>("address", "0.0.0.0".to_string()).unwrap();
        ret
    }
}

impl<T> StreamProcessor for TcpSender<T> 
where T: 'static + Send + Clone
{
    fn init(&mut self) -> Result<(), StreamingError > {
        if self.check_state(StreamingState::Running) {
            self.set_state(StreamingState::Stopped);
            return  Err(StreamingError::InvalidStateTransition);
        }
        if !self.is_initialized() {
            return  Err(StreamingError::InvalidStatics);
        }
        let port = self.get_statics::<u16>("port").expect("").get_value();
        let address = self.get_statics::<String>("address").expect("").get_value();
        match TcpStream::connect(format!("{}:{}", address, port)) {
            Ok(tcp_stream) => {self.tcp_stream = Some(tcp_stream);}
            Err(_) => {
                self.set_state(StreamingState::Stopped);
                return Err(StreamingError::SendDataError);
            }
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamingError> {
        if self.tcp_stream.is_none() {
            self.set_state(StreamingState::Stopped);
            return Err(StreamingError::SendDataError);
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
                    return Err(StreamingError::SendDataError);
                }
            }
            Err(e) => {return Err(e);}
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct TcpMessage<T> {
    pub id_stream: u32,
    pub message: T,
}

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
    phantom:    PhantomData<T>,
    pub logger:     Logger,
    tcp_listen: Option<TcpListener>,
    tcp_stream: HashMap<u32, std::net::TcpStream>,
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
            phantom: PhantomData,
            logger: Logger::new(Some(name)),
            tcp_listen: None,
            tcp_stream: HashMap::new(),
            tcp_handle: Vec::new(),
        };
        ret.new_input::<TcpMessage<T>>("response").unwrap();
        ret.new_output::<TcpMessage<T>>("received").unwrap();
        ret.new_statics::<u16>("port", 50000).unwrap();
        ret.new_statics::<String>("address", "0.0.0.0".to_string()).unwrap();
        ret
    }
    pub fn handle_client(mut stream: TcpStream,
                        stream_id: u32,
                        sender: Output<TcpMessage<T>>) {
        loop {
            let mut buffer = [0; 65535];
            match stream.read(&mut buffer) {
                Ok(n) => {
                    let data = unsafe{from_bytes::<T>(&buffer[0..n])};
                    match data {
                        Ok(data) => {
                            let message = TcpMessage {
                                id_stream: stream_id as u32,
                                message: data.clone(),
                            };
                            let _ = sender.send(message);
                        }
                        Err(_) => {
                            if stream.write_all("Invalid format\n".as_bytes()).is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Server: Errore nella lettura dal flusso: {}", e);
                }
            }
            if *THREAD_EXIT.get_or_init(|| Arc::new(Mutex::new(false))).lock().unwrap() == true {
                break;
            }
        }
    }
}

impl<T> StreamProcessor for TcpReceiver<T> 
where T: 'static + Send + Clone
{
    fn init(&mut self) -> Result<(), StreamingError > {
        if self.check_state(StreamingState::Running) {
            self.set_state(StreamingState::Stopped);
            return  Err(StreamingError::InvalidStateTransition);
        }
        if !self.is_initialized() {
            return  Err(StreamingError::InvalidStatics);
        }
        let port = self.get_statics::<u16>("port").expect("").get_value();
        let address = self.get_statics::<String>("address").expect("").get_value();
        match TcpListener::bind(format!("{}:{}", address, port)) {
            Ok(tcp_listen) => {self.tcp_listen = Some(tcp_listen);}
            Err(_) => {
                self.set_state(StreamingState::Stopped);
                return Err(StreamingError::SendDataError);
            }
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }
    fn run(&mut self) -> Result<(), StreamingError> {
        self.set_state(StreamingState::Running);
        if self.tcp_listen.is_none() {
            self.set_state(StreamingState::Stopped);
            return Err(StreamingError::SendDataError);
        }
        let mut counter_stream: u32 = 0;
        for stream in self.tcp_listen.as_ref().unwrap().incoming() {
            if stream.is_ok() {
                counter_stream += 1;
                log!(self.logger, LogLevel::Info, self.name, "New connection.");
                let _lock = self.lock.lock().unwrap();
                let output = self.get_output::<TcpMessage<T>>("received").expect("").clone();
                let mut tm = TaskManager::get().lock().unwrap();
                let name = self.name;
                let stream = stream.unwrap();
                let stream_thread = stream.try_clone().unwrap();
                let handle = tm.create_task(name, move|| {
                    Self::handle_client(stream_thread, counter_stream, output);
                });
                let stream_vect = stream.try_clone().unwrap();
                match handle {
                    Ok(handle) => {
                        self.tcp_handle.push(handle);
                        self.tcp_stream.insert(counter_stream, stream_vect);
                    }
                    Err(e) => {}
                }
            }
        }
        Ok(())
    }
    fn process(&mut self) -> Result<(), StreamingError > {
        let input: Result<TcpMessage<T>, StreamingError> = self.recv_input::<TcpMessage<T>>("response");
        match input {
            Ok (message) => {
                if let Some(mut stream) = self.tcp_stream.get(&message.id_stream) {
                    let _ = stream.write_all(as_byte::<T>(&message.message));
                }
            },
            Err(e) => {return Err(e);},
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), StreamingError > {
        THREAD_EXIT.get_or_init(|| Arc::new(Mutex::new(true)));
        for j in self.tcp_handle.drain(..) {
            let _ = j.join();
        }
        self.set_state(StreamingState::Stopped);
        Ok(())
    }
}

static THREAD_EXIT: OnceLock<Arc<Mutex<bool>>> = OnceLock::new();