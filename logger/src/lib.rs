use std::ffi::c_char;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, thread};
use std::any::Any;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::Sender;
use chrono::prelude::*;
use std::process::Command;
use std::string::ToString;
use stream_proc_macro::StreamBlockMacro;

use processor_engine::connectors::{ConnectorsTrait, Input, Parameter, Output};
use processor_engine::{create_input, create_output, create_parameter};
use processor_engine::stream_processor::{StreamBlockDyn, StreamProcessor, StreamingError};
use data_model::modules::{ModuleStruct, Version};
use processor_engine::stream_processor::{StreamBlock, StreamingState};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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

pub static MODULE: ModuleStruct  = ModuleStruct {
    name: "logger",
    description: "Log service",
    authors: "Sofia Silvestri",
    release_date: "",
    version: Version{ major: 0,minor: 1,build: 0},
    dependency_number: 0,
    dependencies: &[],
    provides_number: 1,
    provides: &["logger"],
};

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
    parameters: HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    inputs:     HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    state: Arc<Mutex<StreamingState>>,
    log_file_name: String,
    lock: Option<Arc<Mutex<File>>>,
    log_time_start: DateTime<Utc>,
}

impl Logger {
    pub fn new(file_path: &str,
           log_level: LogLevel,
           file_prefix: Option<String>,
           file_suffix: Option<String>) -> Self {
        let l_file_prefix = file_prefix.unwrap_or("KappaStreamLog".to_string());
        let l_file_suffix = file_suffix.unwrap_or(".log".to_string());
        let mut parameters: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        parameters.insert("log_file_path", create_parameter!(String,"log_file_path", "Path for log files", file_path.to_string()));
        parameters.insert("log_file_prefix", create_parameter!(String,"log_file_prefix", "Prefix for log file", l_file_prefix.to_string()));
        parameters.insert("log_file_suffix", create_parameter!(String,"log_file_suffix", "Suffix for log file", l_file_suffix.to_string()));
        parameters.insert("log_level", create_parameter!(LogLevel,"log_level", "Level of log", log_level));
        parameters.insert("log_rotate", create_parameter!(bool,"log_rotate", "Enable log rotation", false));
        parameters.insert("log_compress", create_parameter!(bool,"log_compress", "Enable log compress", false));
        parameters.insert("size_rotate", create_parameter!(u64,"size_rotate", "Set log file size to force rotate", 0));
        parameters.insert("time_rotate", create_parameter!(u64,"time_rotate", "Set time to rotate log", 0));

        let mut inputs: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        inputs.insert("log_entry",create_input!(LogEntry, "log_entry", "Receive log entry streaming"));
        let mut outputs: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        outputs.insert("log_redirect", create_output!(LogEntry, "log_redirect", "Send log entry to other services"));
        let state = Arc::new(Mutex::new(StreamingState::Null));
        let log_time_start = Utc::now();

        Self {
            parameters,
            inputs,
            outputs,
            state,
            log_file_name: "".to_string(),
            lock: None,
            log_time_start,
        }
    }

    fn get_file_path(&self) -> String {
        let date_str = Utc::now().to_rfc3339().to_string();
        let file_name = format!("{}/{}_{}.{}",
                                self.get_parameter_value::<String>("log_file_path").unwrap(),
                                self.get_parameter_value::<String>("log_file_prefix").unwrap(),
                                date_str,
                                self.get_parameter_value::<String>("log_file_suffix").unwrap());
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
        File::create(self.log_file_name.as_str())?;
        Ok(())
    }

    fn rotate_log_file(&mut self) -> Result<(), std::io::Error> {
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
        let binding = self.get_parameter_value::<String>("file_path").unwrap();
        let path = Path::new(binding.as_str());
        if !path.exists() {
            let res = fs::create_dir(binding.as_str());
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
        let input = self.inputs.get("log_entry")
                                    .unwrap()
                                    .as_any()
                                    .downcast_ref::<Input<LogEntry>>()
                                    .unwrap();
        let log_entry: LogEntry = input.recv();
        if log_entry.level < self.get_parameter_value::<LogLevel>("log_level").unwrap() {
            let log_string = format!("{}[{}]: {}\n",
                                        log_entry.time,
                                        log_entry.module,
                                        log_entry.message);
            let res = self.lock
                .clone()
                .unwrap()
                .lock()
                .unwrap()
                .write_all(log_string.as_bytes());
            match res {
                Ok(_) => {}
                Err(_) => {
                    self.set_state(StreamingState::Stopped);
                    return Err(StreamingError::WriteError);
                }
            }
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), StreamingError> {
        // Implementazione specifica per Logger
        self.set_state(StreamingState::Stopped);
        thread::sleep(std::time::Duration::from_secs(1));
        self.lock = None;
        Ok(())
    }
}

fn control(logger: &mut Logger) {
    logger.rotate_log_file().unwrap();
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preliminary_test() {
        let mut logger = Logger::new("logs", LogLevel::Warning, None, None);
        assert_eq!(logger.get_parameter_value::<String>("log_file_path").unwrap(), "logs".to_string());
        assert_eq!(logger.get_parameter_value::<LogLevel>("log_level").unwrap(), LogLevel::Warning);
    }
    fn main() {
        let mut logger = Logger::new("logs", LogLevel::Warning, None, None);
        assert!(logger.init().is_ok());
        let run_thread = {
            let mut logger_clone = logger.clone();
            thread::spawn(move || {
                logger_clone.run().unwrap();
            })
        };
        thread::sleep(std::time::Duration::from_secs(2));
        log!(logger, LogLevel::Info, "Main", "This is an info message.");
        log!(logger, LogLevel::Error, "Main", "This is an error message.");
        assert!(logger.stop().is_ok());
        assert!(run_thread.join().is_ok());
    }
}