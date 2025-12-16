
use std::sync::{Mutex, OnceLock, Arc};
use std::{collections::HashMap, thread::JoinHandle};
use data_model::{memory_manager::MemoryManager, streaming_data::StreamErrCode};
use crate::task_monitor::TaskManager;
use crate::stream_processor::StreamProcessor;
pub struct ProcessorNode {
    pub processor: Box<dyn StreamProcessor>,
    pub next_node: Option<Box<ProcessorNode>>,
    pub prev_node: Option<*mut ProcessorNode>,
}
#[derive(Clone)]
pub struct ProcessorChain {
    pub name: String,
    pub head: Option<*mut ProcessorNode>,
    pub tail: Option<*mut ProcessorNode>,
    pub nodes: Vec<*mut ProcessorNode>,
}

impl ProcessorChain {
    pub fn new(name: String) -> Self {
        ProcessorChain {
            name,
            head: None,
            tail: None,
            nodes: Vec::new(),
        }
    }
    pub fn add_processor(&mut self, processor: Box<dyn StreamProcessor>) {
        let mut new_node = Box::new(ProcessorNode {
            processor,
            next_node: None,
            prev_node: None,
        });
        let new_node_ptr: *mut ProcessorNode = &mut *new_node;

        match self.tail {
            Some(tail_ptr) => unsafe {
                (*tail_ptr).next_node = Some(new_node);
                (*new_node_ptr).prev_node = Some(tail_ptr);
            },
            None => {
                self.head = Some(new_node_ptr);
            }
        }

        self.tail = Some(new_node_ptr);
        self.nodes.push(new_node_ptr);
    }
    pub fn process(&mut self) -> Result<(), StreamErrCode> {
        let mut current_node_ptr = self.head;

        while let Some(node_ptr) = current_node_ptr {
            unsafe {
                let node = &mut *node_ptr;
                node.processor.process()?;
                current_node_ptr = match &node.next_node {
                    Some(next_node) => Some(&**next_node as *const ProcessorNode as *mut ProcessorNode),
                    None => None,
                };
            }
        }

        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), StreamErrCode> {
        for &node_ptr in &self.nodes {
            unsafe {
                let node = &mut *node_ptr;
                node.processor.stop()?;
            }
        }  
        Ok(())
    }
}

unsafe impl Send for ProcessorChain {}
unsafe impl Sync for ProcessorChain {}
pub struct ChainNode {
    pub processor: Box<ProcessorChain>,
    pub next_node: Option<*mut ChainNode>,
    pub prev_node: Option<*mut ChainNode>,
}
#[derive(Clone)]
pub struct ProcessorMode {
    pub name: String,
    pub chains: Vec<ProcessorChain>,
}

impl ProcessorMode {
    pub fn new(name: &str) -> Self {
        ProcessorMode {
            name: name.to_string(),
            chains: Vec::new(),
        }
    }
    pub fn add_chain(&mut self, chain: Box<ProcessorChain>) {
        self.chains.push(*chain);
    }
    pub fn run(&mut self) -> Result<(), StreamErrCode> {
        let mut handles = Vec::new();
        let mut tm = TaskManager::get().lock().unwrap();
        for mut chain in self.chains.clone().into_iter() {
            let handle = tm.create_task(chain.name.clone(), move|| {
                loop {
                    chain.process().unwrap();
                }
            });
            handles.push(handle.unwrap());
        }
        for handle in handles.drain(..) {
            handle.join().unwrap();
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), StreamErrCode> {
        for chain in self.chains.iter_mut() {
            chain.stop()?;
        }
        Ok(())
    }
}

pub struct ProcessorManager {
    pub modes: HashMap<usize, ProcessorMode>,
    pub current_mode_index: usize,
    pub curr_mode_handle: Option<JoinHandle<()>>,
}
impl ProcessorManager {
    pub fn new() -> Self {
        ProcessorManager {
            modes: HashMap::new(),
            current_mode_index: 0,
            curr_mode_handle: None,
        }
    }
    pub fn add_mode(&mut self, mode: ProcessorMode) {
        let index = self.modes.len();
        self.modes.insert(index, mode);
        MemoryManager::get_memory_manager().unwrap().add_mode(index);
    }
    pub fn switch_mode(&mut self, index: usize) -> Result<(), String> {
        if self.current_mode_index == index {
            return Ok(());
        }
        if self.modes.contains_key(&index) {
            if let Some(handle) = self.curr_mode_handle.take() {
            // Stop current mode
                let mut curr_mode = self.modes.get_mut(&self.current_mode_index).unwrap().clone();
                curr_mode.stop().unwrap();
                handle.join().unwrap();
            }
            // Switch memory manager
            MemoryManager::get_memory_manager().unwrap().set_mode(self.current_mode_index);
            // Start new mode
            self.current_mode_index = index;
            let mut new_mode = self.modes.get_mut(&self.current_mode_index).unwrap().clone();
            let mut tm = TaskManager::get().lock().unwrap();
            self.curr_mode_handle  = Some(tm.create_task( new_mode.name.clone(), move || {
                new_mode.run().unwrap();
            }).unwrap());
            Ok(())
        } else {
            Err(format!("Mode with index {} does not exist.", index))
        }
    }
}


pub struct ProcessorEngine {
    processor_map: HashMap<&'static str, Box<dyn StreamProcessor>>,
}

impl ProcessorEngine {
    fn new() -> Self {
        Self { processor_map: HashMap::new() }
    }
    pub fn get() -> &'static Mutex<ProcessorEngine> {
        PROCESSOR_ENGINE.get_or_init(|| Arc::new(Mutex::new(ProcessorEngine::new())))
    }
    pub fn register_processor(&mut self, name: &'static str, processor: Box<dyn StreamProcessor>) -> Result<(), StreamErrCode> {
        if self.processor_map.contains_key(name) {
            return Err(StreamErrCode::AlreadyDefined);
        }
        self.processor_map.insert(name, processor);
        Ok(())
    }
    pub fn init(&mut self) -> Result<(), StreamErrCode>{
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
    pub fn stop(&mut self) -> Result<(), StreamErrCode>{
        for (_, value) in self.processor_map.iter_mut() {
            value.stop()?;
        }
        Ok(())
    }

    pub fn execute_command(&mut self, processor_name: &str, command: &str, args: Vec<&str>) -> Result<String, StreamErrCode> {
        match self.processor_map.get_mut(processor_name) {
            Some(processor) => {
                processor.execute_command(command, args)
            },
            None => Err(StreamErrCode::InvalidInput),
        }
    }
}

static PROCESSOR_ENGINE: OnceLock<Arc<Mutex<ProcessorEngine>>> = OnceLock::new();

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::TestBlock;
    use crate::stream_processor::{StreamBlock};
    #[test]
    fn test_engine() {
        let mut engine = ProcessorEngine::new();
        let mut test_block = TestBlock::new("test_processor");
        test_block.set_statics_value("sum_value", 10).unwrap();
        engine.register_processor("test_processor", Box::new(test_block)).unwrap();
        engine.init().unwrap();
        engine.stop().unwrap();
    }
}