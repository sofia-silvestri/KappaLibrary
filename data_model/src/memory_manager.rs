use std::collections::HashMap;
use std::any::Any;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::fmt::Debug;

use memory_var_macro::MemoryVarMacro;
use crate::streaming_data::StreamErrCode;

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

#[derive(MemoryVarMacro, Clone)]
pub struct Statics<T: 'static + Sync + Send + Debug> {
    pub header: DataHeader,
    value: T,
    limits: Option<[T; 2]>,
    settable: bool,
    lock: Arc<Mutex<()>>,
}

impl<T> Statics<T> 
where T: 'static + Sync + Send + PartialOrd + PartialEq + Debug + Clone
{
    pub fn new(name: &'static str, value: T, limits: Option<[T; 2]>) -> Self {
        let mm= MemoryManager::get_memory_manager();
        let res = Self {
            header: DataHeader{name},
            value,
            limits,
            settable: true,
            lock: Arc::new(Mutex::new(())),
        };
        match mm {
            Ok(mut mgr) => {
                let _ = mgr.get_memory_current_mode().unwrap().register_statics(name, Box::new(res.clone()));
            }
            Err(_) => {}
        }
        res
    }
    pub fn set_value(&mut self, value: T) -> Result<(), StreamErrCode> {
        let _locked = self.lock.lock().unwrap();
        if self.settable {
            self.value = value;
            let mm= MemoryManager::get_memory_manager();
            match mm {
                Ok(mut mgr) => {
                    mgr.get_memory_current_mode().unwrap().update_statics(self.header.name, Box::new(self.clone()));
                }
                Err(e) => {
                    return Err(e);
                }
            }
            self.settable = false;
            Ok(())
        } else {
            Err(StreamErrCode::InvalidOperation)
        }
    }
    pub fn get_value(&self) -> T {
        self.value.clone()
    }
}

impl<T> StaticsTrait for Statics<T> 
where T: 'static + Sync + Send + Debug
{
    fn is_settable(&self) -> bool {
        self.settable
    }
}

#[derive(MemoryVarMacro)]
pub struct State<T: 'static +Send + Sync + Debug> {
    pub header: DataHeader,
    value: T,
    senders: Vec<SyncSender<T>>,
    lock: Arc<Mutex<()>>,
}

impl<T> State<T> where T: 'static + Send + Sync + Clone + PartialOrd + PartialEq + Debug
{
    pub fn new(name: &'static str, value: T) -> Self {
        let mm= MemoryManager::get_memory_manager();
        let res = Self {
            header: DataHeader{name},
            value,
            senders: Vec::new(),
            lock: Arc::new(Mutex::new(())),
        };
        match mm {
            Ok(mut mgr) => {
                mgr.get_memory_current_mode().unwrap().register_state(name, Box::new(res.clone())).unwrap();
            }
            Err(_) => {}
        }
        res
    }
    pub fn set_value(& mut self, value: T) -> Result<(), StreamErrCode> {
        let _locked = self.lock.lock().unwrap();
        self.value = value;
        let mm= MemoryManager::get_memory_manager();
        match mm {
            Ok(mut mgr) => {
                mgr.get_memory_current_mode().unwrap().update_state(self.header.name, Box::new(self.clone()));
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }
    pub fn get_value(&self) -> T {
        let _locked = self.lock.lock().unwrap();
        self.value.clone()
    }
    pub fn connect(&mut self, sender: SyncSender<T>) {
        self.senders.push(sender);
    }
    pub fn send(&self) {
        for s in &self.senders {
            let _ = s.send(self.value.clone());
        }
    }
}
impl<T> Clone for State<T> where T: 'static + Send + Sync + Clone + Debug{
    fn clone(&self) -> Self {
        Self {
            header: DataHeader{name: self.header.name},
            value: self.value.clone(),
            senders: self.senders.clone(),
            lock: self.lock.clone(),
        }
    }
}

#[derive(MemoryVarMacro)]
pub struct Parameter<T:'static + Send + Sync + Clone + Debug> {
    pub header: DataHeader,
    pub value: T,
    pub default: T,
    pub limits: Option<[T; 2]>,
    lock: Arc<Mutex<()>>,
}

impl<T> Parameter<T> where T:'static +  Send + Sync + Clone + PartialOrd + Debug{
    pub fn new(name: &'static str, value: T, limits: Option<[T; 2]>) -> Self {
        let default = value.clone();
        let res = Self {
            header: DataHeader{name},
            value: value,
            default: default,
            limits: limits,
            lock: Arc::new(Mutex::new(())),
        };
        let mm= MemoryManager::get_memory_manager();
        match mm {
            Ok(mut mgr) => {
                mgr.get_memory_current_mode().unwrap().register_parameters(name, Box::new(res.clone())).unwrap();
            }
            Err(_) => {}
        }
        res
    }

    pub fn get_value(&self) -> T {
        let _locked = self.lock.lock().unwrap();
        self.value.clone()

    }
    pub fn set_value(&mut self, value: T) -> Result<(), StreamErrCode> {
        if let Some(limits) = &self.limits {
            if value < limits[0] || value > limits[1] {
                return Err(StreamErrCode::OutOfRange);
            }
        }
        let _locked = self.lock.lock().unwrap();
        self.value = value;
        let mm= MemoryManager::get_memory_manager();
        match mm {
            Ok(mut mgr) => {
                mgr.get_memory_current_mode().unwrap().update_parameters(self.header.name, Box::new(self.clone()));
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }
}

impl<T> Clone for Parameter<T> where T: Send + Sync + Clone + Debug {
    fn clone(&self) -> Self {
        Self {
            header: DataHeader{name: self.header.name},
            value: self.value.clone(),
            default: self.default.clone(),
            limits: self.limits.clone(),
            lock: self.lock.clone(),
        }
    }
}
pub struct MemoryMode {
    mapped_state:       HashMap<&'static str, Box<dyn DataTrait>>,
    mapped_statics:     HashMap<&'static str, Box<dyn DataTrait>>,
    mapped_parameters:  HashMap<&'static str, Box<dyn DataTrait>>
}

impl MemoryMode {
    pub fn new() -> Self {
        Self {
            mapped_state: HashMap::new(),
            mapped_statics: HashMap::new(),
            mapped_parameters: HashMap::new(),
        }
    }
    pub fn register_state(&mut self, key: &'static str, state: Box<dyn DataTrait>) -> Result<(), StreamErrCode> {
        if self.mapped_state.contains_key(key) {
            return Err(StreamErrCode::AlreadyDefined);
        }
        self.mapped_state.insert(key, state);
        Ok(())
    }
    pub fn register_statics(&mut self, key: &'static str, statics: Box<dyn DataTrait>) -> Result<(), StreamErrCode> {
        if self.mapped_statics.contains_key(key) {
            return Err(StreamErrCode::AlreadyDefined);
        }
        self.mapped_statics.insert(key, statics);
        Ok(())
    }
    pub fn register_parameters(&mut self, key: &'static str, param: Box<dyn DataTrait>) -> Result<(), StreamErrCode> {
        if self.mapped_parameters.contains_key(key) {
            return Err(StreamErrCode::AlreadyDefined);
        }
        self.mapped_parameters.insert(key, param);
        Ok(())
    }
    pub fn update_state(&mut self, key: &'static str, state: Box<dyn DataTrait>) {
        self.mapped_state.insert(key, state);
    }
    pub fn update_statics(&mut self, key: &'static str, statics: Box<dyn DataTrait>) {
        self.mapped_statics.insert(key, statics);
    }
    pub fn update_parameters(&mut self, key: &'static str, param: Box<dyn DataTrait>) {
        self.mapped_parameters.insert(key, param);
    }
    pub fn serialize_all(&self) -> String {
        let mut result = String::new();
        result.push_str("{\"memory_mapped\":");
        result.push_str("{\"state\": {");
        for (_, val) in &self.mapped_state {
            result.push_str(&format!("{}", val.serialize()));
        }

        result.push_str("}");
        result.push_str(",{\"statics\": {");
        for (_, val) in &self.mapped_statics {
            result.push_str(&format!("{}", val.serialize()));
        }
        result.push_str("}");
        result.push_str(",{\"parameters\": {");
        for (_, val) in &self.mapped_parameters {
            result.push_str(&format!("{}", val.serialize()));
        }
        result.push_str("}}");
        result
        
    }
}

pub struct MemoryManager {
    memory_modes: HashMap<usize, MemoryMode>,
    current_mode_index: usize,
}

impl MemoryManager {
    fn new() -> Self {
        MemoryManager {
            memory_modes: HashMap::new(),
            current_mode_index: 0,
        }
    }
    pub fn get_memory_manager() -> Result<MutexGuard<'static, MemoryManager>, StreamErrCode> {
        let manager = MemoryManager::get_instance();
        match manager.lock() {
            Ok(guard) => Ok(guard),
            Err(_) => Err(StreamErrCode::GenericError),
        }
    }
    pub fn get_instance() -> &'static Mutex<MemoryManager> {
        MEMORY_MANAGER.get_or_init( || Mutex::new(MemoryManager::new()))
    }
    pub fn add_mode(&mut self, index: usize) {
        self.memory_modes.insert(index, MemoryMode::new());
    }
    pub fn set_mode(&mut self, index: usize) {
        self.current_mode_index = index;
    }
    pub fn get_memory_current_mode(&mut self) -> Option<&mut MemoryMode> {
        self.memory_modes.get_mut(&self.current_mode_index)
    }
    pub fn get_memory_mode(&mut self, index: usize) -> Option<&mut MemoryMode> {
        self.memory_modes.get_mut(&index)
    }
}

pub static MEMORY_MANAGER: OnceLock<Mutex<MemoryManager>> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager() {
        match MemoryManager::get_memory_manager() {
            Ok(_) => {
                assert!(true);
            }
            Err(_) => panic!("Failed to lock MemoryManager"),
        };
    }
    #[test]
    fn test_static_variable() {
        let mut statics = Statics::new("test_statics", 10);
        assert_eq!(statics.get_value(), 10);
        statics.set_value(20).unwrap();
        assert_eq!(statics.get_value(), 20);
        let result = statics.set_value(30);
        assert!(result.is_err());
    }
    #[test]
    fn test_state_variable() {
        let mut state = State::new("test_state", 10);
        assert_eq!(state.get_value(), 10);
        state.set_value(20).unwrap();
        assert_eq!(state.get_value(), 20);
        let result = state.set_value(30);
        assert!(result.is_ok());
    }
    #[test]
    fn test_parameter_variable() {
        let mut param = Parameter::new("test_param", 10, Some([10, 20]));
        assert_eq!(param.get_value(), 10);
        param.set_value(20).unwrap();
        assert_eq!(param.get_value(), 20);
        let result = param.set_value(30);
        assert!(result.is_err());
        assert_eq!(param.get_value(), 20);
    }
    #[test]
    fn test_memory_manager_serialization() {
        use std::io::Write;
        use std::fs;
        use std::path::Path;

        let _ = Statics::new("test_statics_reg", 10);
        let _ = State::new("test_state_reg", 20);
        let _ = Parameter::new("test_param_reg", 15, Some([10, 20]));
        let mm = MemoryManager::get_memory_manager().unwrap();
        let serialized = mm.serialize_all();
        assert!(serialized.contains("\"test_statics_reg\""));
        assert!(serialized.contains("\"test_state_reg\""));
        assert!(serialized.contains("\"test_param_reg\""));
        let path = Path::new("test_memory_manager_serialization.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        let json_result = serde_json::from_str::<serde_json::Value>(&serialized);
        //assert!(json_result.is_ok());
    }
}