use std::collections::HashMap;
use std::any::Any;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

trait StateTrait: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn serialize(&self) -> Result<String, String>;
}

#[derive(Clone, Copy, Serialize)]
pub struct State<T: 'static + Send> {
    data: T,
}

impl<T> State<T> where T: Send {
    pub fn new(data:T) -> Self {
        Self { data }
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

pub struct MemoryManager {
    mapped_memory: HashMap<&'static str, Box<dyn StateTrait>>,
}

impl MemoryManager {
    fn new() -> Self {
        Self {
            mapped_memory: HashMap::new()
        }
    }

    pub fn get() -> &'static Mutex<MemoryManager> {
        MEMORY_MANAGER.get_or_init( || Mutex::new(MemoryManager::new()))
    }
    pub fn register_state<T:Send + Copy+ Sync + Serialize>(&mut self, key: &'static str, value: T)
        -> State<T>
    {
        let mm = MemoryManager::get();
        let state = State::<T>::new(value);
        let boxed = Box::new(state);
        mm.lock().unwrap().mapped_memory.insert(key, boxed);
        state
    }

    pub fn serialize_all() -> Result<String, String>{
        let mm = MemoryManager::get();
        let mut json_string: String = "".to_string();
        for m in mm.lock().unwrap().mapped_memory.iter() {
            match m.1.serialize() {
                Ok(json) => {json_string += &json;}
                Err(e) => {return Err(format!("{}", e));}
            }
        }
        Ok(json_string)
    }
}

static MEMORY_MANAGER: OnceLock<Mutex<MemoryManager>> = OnceLock::new();