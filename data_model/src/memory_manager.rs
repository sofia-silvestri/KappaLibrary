use std::collections::HashMap;
use std::any::Any;
use std::sync::{Mutex, OnceLock};
use std::fmt::Display;

use serde::Serialize;

use crate::streaming_error::{StreamingState, StreamingError};

pub trait StaticsTrait : Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> Option<&mut dyn Any>;
    fn is_settable(&self) -> bool;
}

#[derive(Copy, Clone, Debug)]
pub struct Statics<T: 'static> {
    key: &'static str,
    value: T,
    settable: bool,
}
impl<T: 'static> StaticsTrait for Statics<T> 
where T: Send + Sync
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        if self.settable {
            Some(self)
        } else {
            None
        }
    }
    fn is_settable(&self) -> bool {
        self.settable
    }
}

impl<T> Display for Statics<T>
where
    T: 'static + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: 'static + Send + Sync + Copy + Serialize> Statics<T> {
    pub fn new(key: &'static str, value: T) -> Self {
        MemoryManager::get().lock().unwrap().register_state::<T>(key, value);
        Self { key, value, settable: true }
    }
    pub fn set(&mut self, value: T) -> Result<(), StreamingError> {
        if self.settable {
            self.value = value;
            self.settable = false;
            MemoryManager::get().lock().unwrap().register_state::<T>(self.key, self.value);
            Ok(())
        } else {
            Err(StreamingError::InvalidOperation)
        }
    }
    pub fn get(&self) -> &T {
        &self.value
    }
}

trait StateTrait: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn serialize(&self) -> Result<String, String>;
}

#[derive(Clone, Copy, Serialize)]
pub struct State<T: 'static + Send> {
    name: &'static str,
    data: T,
}

impl<T> State<T> where T: Send {
    pub fn new(name: &'static str, data:T) -> Self {
        Self { name, data }
    }
    
}

impl<T: Send + Sync+ Serialize> StateTrait for State<T>{
    fn as_any(&self) -> &dyn Any {self}
    fn as_any_mut(&mut self) -> &mut dyn Any {self}
    fn serialize(&self) -> Result<String, String> {
        match serde_json::to_string(&self) {
            Ok(json_string) => {
                Ok(json_string)
            }
            Err(e) => {
                Err(format!("Serialization Error. {}",e))
            }
        }
    }
}

pub trait StreamBlockDyn : Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn check_state(&self, state: StreamingState) -> bool;
    fn set_state(&mut self, state: StreamingState);
    fn get_input_list(&self) -> Vec<&str>;
    fn get_output_list(&self) -> Vec<&str>;
    fn get_parameter_list(&self) -> Vec<&str>;
    fn get_statics_list(&self) -> Vec<&str>;
    fn is_initialized(&self) -> bool;
    fn get_qualified_name(&self, name: &str) -> &'static str;
}


pub struct MemoryManager {
    mapped_state:       HashMap<&'static str, Box<dyn StateTrait>>,
    mapped_statics:     HashMap<&'static str, Box<dyn StaticsTrait>>,
    mapped_proc_block:  HashMap<&'static str, Box<dyn StreamBlockDyn>>
}

impl MemoryManager {
    fn new() -> Self {
        Self {
            mapped_state:       HashMap::new(),
            mapped_statics:     HashMap::new(),
            mapped_proc_block:  HashMap::new()
        }
    }

    pub fn get() -> &'static Mutex<MemoryManager> {
        MEMORY_MANAGER.get_or_init( || Mutex::new(MemoryManager::new()))
    }
    pub fn register_state<T: Send + Copy+ Sync + Serialize>(&mut self, key: &'static str, value: T)
        -> State<T>
    {
        let mm = MemoryManager::get();
        let state = State::<T>::new(key, value);
        let boxed = Box::new(state);
        mm.lock().unwrap().mapped_state.insert(key, boxed);
        state
    }

    pub fn register_static<T: Send + Copy+ Sync + Serialize>(&mut self, key: &'static str, value: T) -> Statics<T>
    {
        let mm = MemoryManager::get();
        let statics = Statics::<T>::new(key, value);
        let boxed = Box::new(statics);
        mm.lock().unwrap().mapped_statics.insert(key, boxed);
        statics
    }
    pub fn register_module<T: Send + Copy+ Sync + Serialize>(&mut self, key: &'static str, block: Box<dyn StreamBlockDyn>)
    {
        let mm = MemoryManager::get();
        mm.lock().unwrap().mapped_proc_block.insert(key, block);
        
    }
    pub fn serialize_all() -> Result<String, String>{
        let mm = MemoryManager::get();
        let mut json_string: String = "".to_string();
        for m in mm.lock().unwrap().mapped_state.iter() {
            match m.1.serialize() {
                Ok(json) => {json_string += &json;}
                Err(e) => {return Err(format!("{}", e));}
            }
        }
        Ok(json_string)
    }
}

static MEMORY_MANAGER: OnceLock<Mutex<MemoryManager>> = OnceLock::new();