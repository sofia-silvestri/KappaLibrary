use core::task;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::fmt;
use chrono::{DateTime, Utc};
use data_model::streaming_data::StreamingError;
use libc::{clock_gettime, clockid_t, pthread_getcpuclockid, pthread_self, pthread_t, timespec};
use utils::math::statistics::{mean, std_deviation, percentile};

use crate::stream_processor::{StreamBlockDyn, StreamProcessor};

pub struct KappaStatistics {
    pub timestamp: f64,
    pub mean: f64,
    pub max: f64,
    pub min: f64,
    pub std_dev: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
}

pub struct ProcessorEngine {
    processor_map: HashMap<&'static str, &'static Box<dyn StreamProcessor>>,
}

impl ProcessorEngine {
    fn new() -> Self {
        Self { processor_map: HashMap::new() }
    }
    pub fn get() -> &'static Mutex<ProcessorEngine> {
        PROCESSOR_ENGINE.get_or_init(|| Mutex::new(ProcessorEngine::new()))
    }
    pub fn register_processor(&mut self, name: &'static str, processor: &'static Box<dyn StreamProcessor>) -> Result<(), StreamingError> {
        if let Some(_) = self.processor_map.insert(name, processor) {
            Ok(())
        } else {
            Err(StreamingError::AlreadyDefined)
        }
    }
    pub fn init(&mut self) -> Result<(), StreamingError>{
        for (_, value) in self.processor_map.iter_mut() {
            match value.init() {
                Ok(_) => {}
                Err(e) => {
                    let _ = self.stop();
                    return Err(e);
                }
            }
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), StreamingError>{
        for (_, value) in self.processor_map.iter_mut() {
            value.stop()?;
        }
        Ok(())
    }
}

static PROCESSOR_ENGINE: OnceLock<Mutex<ProcessorEngine>> = OnceLock::new();