use std::collections::HashMap;
use std::any::Any;
use std::io::Write;
use std::thread;
use std::process::Command;
use std::fs;
use std::path::Path;
use std::fmt::Display;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use chrono::prelude::*;
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamingError, StreamingState};
use data_model::memory_manager::{DataTrait, StaticsTrait, State, Parameter, Statics};
use crate::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use crate::connectors::{ConnectorTrait, Input, Output};

#[derive(Debug, Clone, PartialOrd, PartialEq, Copy, Serialize)]
pub enum LogLevel {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct LogEntry {
    level: LogLevel,
    module: String,
    message: String,
    time: DateTime<Utc>,
}

impl LogEntry {
    pub fn new(level: LogLevel, module: String, message: String) -> Self {
        Self {
            level,
            module,
            message,
            time: Utc::now(),
        }
    }
}

#[derive(StreamBlockMacro)]
pub struct Logger {
    name:       &'static str,
    inputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters: HashMap<&'static str, Box<dyn DataTrait>>,
    statics:    HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:      HashMap<&'static str, Box<dyn DataTrait>>,
    lock:       Arc<Mutex<()>>,
    proc_state: Arc<Mutex<StreamingState>>,
    log_time_start: DateTime<Utc>,
    log_file_name: String,
    log_file:   Option<fs::File>,
}

impl Logger {
    pub fn new(name: Option<&'static str>) -> Self {
        let logger_name = match name {
            Some(n) => n,
            None => "KappaLogger",
        };
        let mut logger = Self {
            name: logger_name,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            parameters: HashMap::new(),
            statics: HashMap::new(),
            state: HashMap::new(),
            lock: Arc::new(Mutex::new(())),
            proc_state: Arc::new(Mutex::new(StreamingState::Null)),
            log_time_start: Utc::now(),
            log_file_name: String::new(),
            log_file: None,
        };
        logger.new_parameter::<&'static str>("log_file_path", "./log", None).unwrap();
        logger.new_parameter::<&'static str>("log_file_prefix", "", None).unwrap();
        logger.new_parameter::<&'static str>("log_file_suffix", "", None).unwrap();
        logger.new_parameter::<LogLevel>("log_level", LogLevel::Warning, None).unwrap();
        logger.new_parameter::<bool>("log_rotate",  false, None).unwrap();
        logger.new_parameter::<bool>("log_compress", false, None).unwrap();
        logger.new_parameter::<f64>("size_rotate_MB",  500.0, None).unwrap();
        logger.new_parameter::<f64>("time_rotate_sec", 24.0*60.0*60.0, None).unwrap();
        logger.new_input::<LogEntry>("log_entry").unwrap();
        logger.new_output::<LogEntry>("log_redirect").unwrap();
        logger
    }
    fn get_file_path(&self) -> String {
        let date_str = Utc::now().to_rfc3339().to_string();
        let file_name = format!("{}/{}_{}.{}",
                                self.get_parameter_value::<&'static str>("log_file_path").unwrap(),
                                self.get_parameter_value::<&'static str>("log_file_prefix").unwrap(),
                                date_str,
                                self.get_parameter_value::<&'static str>("log_file_suffix").unwrap());
        file_name
    }

    fn start_log_file(&mut self) -> Result<(), std::io::Error> {
        if self.get_parameter_value::<bool>("log_compress").unwrap() {
            if Path::new(self.log_file_name.as_str()).exists() {
                let path = self.log_file_name.clone();
                thread::spawn( move || {
                    Command::new("xz").arg(format!("{}",path));
                });
            }
        }
        self.log_file_name = self.get_file_path();
        self.log_file = Some(fs::File::create(self.log_file_name.as_str())?);
        Ok(())
    }

    pub fn rotate_log_file(&mut self) -> Result<(), std::io::Error> {
        loop {
            thread::sleep(std::time::Duration::from_secs(1));
            let size_rotate = self.get_parameter_value::<u64>("size_rotate").unwrap();
            let time_rotate = self.get_parameter_value::<u64>("time_rotate").unwrap();
            let rotate_enabled = self.get_parameter_value::<bool>("log_rotate").unwrap();
            if rotate_enabled {
                let metadata = fs::metadata(self.log_file_name.as_str())?;
                let file_size = metadata.len();
                let elapsed_time = Utc::now().signed_duration_since(self.log_time_start).num_seconds() as u64;
                if (size_rotate > 0 && file_size >= size_rotate) ||
                   (time_rotate > 0 && elapsed_time >= time_rotate) {
                    self.start_log_file()?;
                    self.log_time_start = Utc::now();
                }
            }
        }
    }
}

impl StreamProcessor for Logger {
    // Implementazione dei metodi del trait StreamProcessor
    fn init(&mut self) -> Result<(), StreamingError> {
        // Implementazione specifica per Logger
        let binding = self.get_parameter_value::<&'static str>("file_path").unwrap();
        let path = Path::new(binding);
        if !path.exists() {
            let res = fs::create_dir(binding);
            match res {
                Ok(_) => {}
                Err(_) => {
                    self.set_state(StreamingState::Stopped);
                    return Err(StreamingError::CreateError);
                }
            }
        } else {
            if !path.is_dir() {
                self.set_state(StreamingState::Stopped);
                return Err(StreamingError::PathError);
            }
        }
        self.set_state(StreamingState::Initial);
        Ok(())
    }

    fn run(&mut self) -> Result<(), StreamingError> {
        // Implementazione specifica per Logger
        self.start_log_file().map_err(|_| StreamingError::CreateError)?;
        self.set_state(StreamingState::Running);
        self.log_time_start = Utc::now();
        while self.check_state(StreamingState::Running) {
            self.process()?;
        }
        Ok(())
    }

    fn process(&mut self) -> Result<(), StreamingError> {
        let mut error: bool = false; 
        let input = self.recv_input::<LogEntry>("log_entry");
        match input {
            Ok(log_entry) => {
                if log_entry.level < self.get_parameter_value::<LogLevel>("log_level").unwrap() {
                    let log_string = format!("{}[{}]: {}\n",
                                                log_entry.time,
                                                log_entry.module,
                                                log_entry.message);
                    let _lock = self.lock.lock().unwrap();
                    let res = self.log_file.as_mut().unwrap().write_all(log_string.as_bytes());
                    match res {
                        Ok(_) => {}
                        Err(_) => {
                            error = true;
                        }
                    }
                }
                if error {
                    self.set_state(StreamingState::Stopped);
                    return Err(StreamingError::WriteError);

                } else {
                    return Ok(());
                }
            }
            Err(e) => {return Err(e);}
        }
    }
    fn stop(&mut self) -> Result<(), StreamingError> {
        self.set_state(StreamingState::Stopped);
        thread::sleep(std::time::Duration::from_secs(1));
        Ok(())
    }
}

#[macro_export]
macro_rules! log {
    ($logger:expr, $level:expr, $module:expr, $message:expr) => {
        {
            let log_entry = LogEntry::new($level, $module.to_string(), $message.to_string());
            let input = $logger.inputs.get("log_entry")
                                        .unwrap()
                                        .as_any()
                                        .downcast_ref::<Input<LogEntry>>()
                                        .unwrap();
            input.send(log_entry);
        }
    };
}