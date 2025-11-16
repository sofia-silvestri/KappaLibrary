use std::collections::HashMap;
use std::any::Any;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use std::fmt::Display;

use memory_var_macro::MemoryVarMacro;
use crate::streaming_data::StreamingError;

// General traits for Statics, States and Parameters
#[derive(Debug, Clone, Copy)]
pub struct DataHeader {
    pub name: &'static str,
}

pub trait DataTrait : Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn get_header(&self) -> &DataHeader;
    fn serialize(&self) -> String;
}

pub trait StaticsTrait : Send + Sync + DataTrait {
    fn is_settable(&self) -> bool;
}

#[derive(MemoryVarMacro)]
pub struct Statics<T: 'static + Sync + Send + Display> {
    pub header: DataHeader,
    value: T,
    settable: bool,
    lock: Arc<Mutex<()>>,
}

impl<T> Statics<T> 
where T: 'static + Sync + Send + Copy + PartialOrd + PartialEq + Display
{
    pub fn new(name: &'static str, value: T) -> Self {
        MEMORY_MANAGER.get()
                    .expect("")
                    .lock()
                    .unwrap()
                    .register_state::<T>(name, value);
        Self {
            header: DataHeader{name},
            value,
            settable: true,
            lock: Arc::new(Mutex::new(())),
        }
    }
    pub fn set_value(&mut self, value: T) -> Result<(), StreamingError> {
        let _locked = self.lock.lock().unwrap();
        if self.settable {
            self.value = value;
            MEMORY_MANAGER.get()
                    .expect("")
                    .lock()
                    .unwrap()
                    .register_state::<T>(self.header.name, self.get_value());
            Ok(())
        } else {
            Err(StreamingError::InvalidOperation)
        }
    }
    pub fn get_value(&self) -> T {
        self.value
    }
}

impl<T> StaticsTrait for Statics<T> 
where T: 'static + Sync + Send + Copy + Display
{
    fn is_settable(&self) -> bool {
        self.settable
    }
}

#[derive(MemoryVarMacro)]
pub struct State<T: 'static +Send + Sync + Display> {
    pub header: DataHeader,
    value: T,
    senders: Vec<Sender<T>>,
    lock: Arc<Mutex<()>>,
}

impl<T> State<T> where T: 'static + Send + Sync + Clone + Copy + PartialOrd + PartialEq + Display
{
    pub fn new(name: &'static str, value: T) -> Self {
        MEMORY_MANAGER.get()
                    .expect("")
                    .lock()
                    .unwrap()
                    .register_state::<T>(name, value);
        Self {
            header: DataHeader{name},
            value,
            senders: Vec::new(),
            lock: Arc::new(Mutex::new(())),
        }
    }
    pub fn set_value(& mut self, value: T) {
        let _locked = self.lock.lock().unwrap();
        self.value = value;
        MEMORY_MANAGER.get()
                    .expect("")
                    .lock()
                    .unwrap()
                    .register_state::<T>(self.header.name, self.get_value());
    }
    pub fn get_value(&self) -> T {
        let _locked = self.lock.lock().unwrap();
        self.value
    }
    pub fn connect(&mut self, sender: Sender<T>) {
        self.senders.push(sender);
    }
    pub fn send(&self) {
        for s in &self.senders {
            let _ = s.send(self.value);
        }
    }
}
impl<T> Clone for State<T> where T: 'static + Send + Sync + Clone + Copy + Display{
    fn clone(&self) -> Self {
        Self {
            header: DataHeader{name: self.header.name},
            value: self.value,
            senders: self.senders.clone(),
            lock: self.lock.clone(),
        }
    }
}

#[derive(MemoryVarMacro)]
pub struct Parameter<T: 'static + Send + Sync + Copy + Clone + Display> {
    pub header: DataHeader,
    pub value: T,
    pub default: T,
    pub limits: Option<[T; 2]>,
    lock: Arc<Mutex<()>>,
}

impl<T> Parameter<T> where T: 'static + Send + Sync + Copy + Clone + PartialOrd + Display{
    pub fn new(name: &'static str, value: T, limits: Option<[T; 2]>) -> Self {
        let default = value.clone();
        MEMORY_MANAGER.get()
                    .expect("")
                    .lock()
                    .unwrap()
                    .register_parameters::<T>(name, value.clone());
        Self {
            header: DataHeader{name},
            value: value,
            default: default,
            limits: limits,
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn get_value(&self) -> T {
        let _locked = self.lock.lock().unwrap();
        self.value.clone()

    }
    pub fn set_value(&'static mut self, value: T) -> Result<(), StreamingError> {
        if let Some(limits) = &self.limits {
            if value < limits[0] || value > limits[1] {
                return Err(StreamingError::OutOfRange);
            }
        }
        let _locked = self.lock.lock().unwrap();
        self.value = value;
        MEMORY_MANAGER.get()
                    .expect("")
                    .lock()
                    .unwrap()
                    .register_parameters::<T>(self.header.name, self.get_value());
        Ok(())
    }
}

impl<T> Clone for Parameter<T> where T: 'static + Send + Sync + Copy + Clone + Display {
    fn clone(&self) -> Self {
        Self {
            header: DataHeader{name: self.header.name},
            value: self.value.clone(),
            default: self.default,
            limits: self.limits,
            lock: self.lock.clone(),
        }
    }
}
pub struct MemoryManager {
    mapped_state:       HashMap<&'static str, Box<dyn DataTrait>>,
    mapped_statics:     HashMap<&'static str, Box<dyn DataTrait>>,
    mapped_parameters:  HashMap<&'static str, Box<dyn DataTrait>>
}

impl MemoryManager {
    fn new() -> Self {
        Self {
            mapped_state: HashMap::new(),
            mapped_statics: HashMap::new(),
            mapped_parameters: HashMap::new(),
        }
    }
    pub fn get() -> &'static Mutex<MemoryManager> {
        MEMORY_MANAGER.get_or_init( || Mutex::new(MemoryManager::new()))
    }
    pub fn register_state<T: 'static + Send + Sync + Copy + Clone + PartialOrd + PartialEq + Display>(&mut self, key: &'static str, value: T  ) {
        self.mapped_state.insert(key, Box::new(State::<T>::new(key, value)));
    }
    pub fn register_statics<T: 'static + Send + Sync + Copy + Clone + PartialOrd + PartialEq + Display>(&mut self, key: &'static str, value: T) {
        self.mapped_statics.insert(key, Box::new(Statics::<T>::new(key, value)));
    }
    pub fn register_parameters<T: 'static + Send + Sync + Copy + Clone + PartialOrd + PartialEq + Display>(&mut self, key: &'static str, value: T) {
        self.mapped_parameters.insert(key, Box::new(Parameter::<T>::new(key, value, None)));
    }
    pub fn serialize_all(&self) -> String {
        let mut result = String::new();
        result.push_str("{\n\"  state\": {\n");
        for (_, val) in &self.mapped_state {
            result.push_str(&format!("    {}\n", val.serialize()));
        }
        result.push_str("}\n");
        result.push_str("{\n\"  statics\": {\n");
        for (_, val) in &self.mapped_statics {
            result.push_str(&format!("    {}\n", val.serialize()));
        }
        result.push_str("}\n");
        result.push_str("{\n\"  parameters\": {\n");
        for (_, val) in &self.mapped_parameters {
            result.push_str(&format!("    {}\n", val.serialize()));
        }
        result.push_str("}\n");
        result
        
    }
}

static MEMORY_MANAGER: OnceLock<Mutex<MemoryManager>> = OnceLock::new();