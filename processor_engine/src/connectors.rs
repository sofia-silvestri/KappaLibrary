use std::any::{Any,TypeId};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use connectors_macro::ConnectorMacro;
use data_model::streaming_error::StreamingError;
use data_model::memory_manager::MemoryManager;
use crate::stream_processor::DataHeader;
use serde::Serialize;

pub trait ConnectorsTrait : 'static + Any + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(ConnectorMacro)]
pub struct Input<T: 'static + Send + Any> {
    pub header: DataHeader,
    pub sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T: 'static + Send + Any> Input<T> {
    pub fn new(name: &'static str, description: &'static str) -> Self{
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            header: DataHeader::new(name, description, TypeId::of::<T>()),
            sender,
            receiver,
        }
    }
    pub fn send(&self, data: T) {
        self.sender.send(data).unwrap();
    }
    pub fn recv(&self) -> T {
        self.receiver.recv().unwrap()
    }
}


#[macro_export]
macro_rules! create_input {
    ($type:ty, $name:expr, $description:expr) => {
        Box::new(
            Input::<$type>::new(
                $name,
                $description,
            )
        )
    };
}

#[derive(ConnectorMacro)]
pub struct Output<T: 'static + Send> {
    pub header: DataHeader,
    pub senders: Vec<Sender<T>>,
}

impl<T: 'static + Send + Any + Clone> Output<T> {
    pub fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            header: DataHeader::new(name, description, TypeId::of::<T>()),
            senders: Vec::new(),
        }
    }
    pub fn connect(&mut self, sender: Sender<T>) {
        self.senders.push(sender);
    }
    pub fn send(&self, data: T) {
        for s in &self.senders {
            let _ = s.send(data.clone());
        }
    }
}
#[macro_export]
macro_rules! create_output {
    // Definizione del pattern di chiamata
    ($type:ty, $name:expr, $description:expr) => {
        // Espansione del codice: Box::new(...)
        Box::new(
            Output::<$type>::new(
                $name,
                $description,
            )
        )
    };
}
#[derive(ConnectorMacro)]
pub struct Parameter<T: 'static + Send + Sync + Copy + Clone + Serialize> {
    pub header: DataHeader,
    pub value: Arc<Mutex<T>>,
    pub default: T,
    pub limits: Option<[T; 2]>,
}

impl<T: 'static + Send + Sync + Copy + Clone + Serialize + PartialOrd> Parameter<T> {
    pub fn new(name: &'static str, description: &'static str, value: T) -> Self {
        let default = value.clone();
        MemoryManager::get().lock().unwrap().register_state::<T>(&name, default);
        Self {
            header: DataHeader::new(&name, &description, TypeId::of::<T>()),
            value: Arc::new(Mutex::new(value)),
            default: default,
            limits: None,
        }
    }

    pub fn get_value(&self) -> T {
        self.value.lock().unwrap().clone()
    }
    pub fn set_value(&mut self, value: T) -> Result<(), StreamingError> {
        if let Some(limits) = &self.limits {
            if value < limits[0] || value > limits[1] {
                return Err(StreamingError::OutOfRange);
            }
        }
        *self.value.lock().unwrap() = value;
        MemoryManager::get().lock().unwrap().register_state::<T>(&self.header.name, value);
        Ok(())
    }
}

#[macro_export]
macro_rules! create_parameter {
    // Definizione del pattern di chiamata
    ($type:ty, $name:expr, $description:expr, $value:expr) => {
        // Espansione del codice: Box::new(...)
        Box::new(
            Parameter::<$type>::new(
                $name,
                $description,
                $value
            )
        )
    };
}