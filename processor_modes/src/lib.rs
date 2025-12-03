use std::collections::HashMap;

use processor_engine::stream_processor::StreamProcessor;
use data_model::{memory_manager::MemoryManager, streaming_data::StreamingError};
pub struct ProcessorNode {
    pub processor: Box<dyn StreamProcessor>,
    pub next_node: Option<Box<ProcessorNode>>,
    pub prev_node: Option<*mut ProcessorNode>,
}
#[derive(Clone)]
pub struct ProcessorChain {
    pub head: Option<*mut ProcessorNode>,
    pub tail: Option<*mut ProcessorNode>,
    pub nodes: Vec<*mut ProcessorNode>,
}

impl ProcessorChain {
    pub fn new() -> Self {
        ProcessorChain {
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
    pub fn process(&mut self) -> Result<(), StreamingError> {
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
    pub fn stop(&mut self) -> Result<(), StreamingError> {
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
    pub fn run(&mut self) -> Result<(), StreamingError> {
        let mut handles = Vec::new();
        for mut chain in self.chains.clone().into_iter() {
            let handle = std::thread::spawn( move|| {
                loop {
                    chain.process().unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles.drain(..) {
            handle.join().unwrap();
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), StreamingError> {
        for chain in self.chains.iter_mut() {
            chain.stop()?;
        }
        Ok(())
    }
}

pub struct ProcessorManager {
    pub modes: HashMap<usize, ProcessorMode>,
    pub current_mode_index: usize,
}
impl ProcessorManager {
    pub fn new() -> Self {
        ProcessorManager {
            modes: HashMap::new(),
            current_mode_index: 0,
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
            // Stop current mode
            let mut curr_mode = self.modes.get_mut(&self.current_mode_index).unwrap().clone();
            curr_mode.stop().unwrap();
            // Switch memory manager
            MemoryManager::get_memory_manager().unwrap().set_mode(self.current_mode_index);
            // Start new mode
            self.current_mode_index = index;
            let mut new_mode = self.modes.get_mut(&self.current_mode_index).unwrap().clone();
            std::thread::spawn( move || {
                new_mode.run().unwrap();
            });
            Ok(())
        } else {
            Err(format!("Mode with index {} does not exist.", index))
        }
    }
}