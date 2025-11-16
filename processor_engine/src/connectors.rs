use std::any::Any;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use data_model::memory_manager::DataHeader;

pub trait ConnectorTrait {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn get_header(&self) -> &DataHeader;
}

pub struct Input<T: 'static + Send + Any> {
    pub header: DataHeader,
    pub sender: Sender<T>,
    receiver: Receiver<T>,
    queue_size: Arc<Mutex<usize>>,
}

impl<T> Input<T> 
where T: 'static + Send + Any
{
    pub fn new(name: &'static str) -> Self{
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            header: DataHeader{name},
            sender,
            receiver,
            queue_size: Arc::new(Mutex::new(0)),
        }
    }
    pub fn send(&mut self, data: T) {
        self.sender.send(data).unwrap();
        let queue_size = *self.queue_size.lock().unwrap();
        *self.queue_size.lock().unwrap() = queue_size+1;
    }
    pub fn recv(&mut self) -> T {
        let queue_size = *self.queue_size.lock().unwrap();
        *self.queue_size.lock().unwrap() = queue_size-1;
        self.receiver.recv().unwrap()
    }
    pub fn get_queue_size(&self) -> usize {
        *self.queue_size.lock().unwrap()
    }
}
impl<T: 'static + Send + Any> ConnectorTrait for Input<T> {
    fn as_any(&self) -> &dyn Any {self}
    fn as_any_mut(&mut self) -> &mut dyn Any {self}
    fn get_header(&self) -> &DataHeader {&self.header}
}

pub struct Output<T: 'static + Send> {
    pub header: DataHeader,
    pub senders: Vec<Sender<T>>,
}

impl<T: 'static + Send + Any + Clone> Output<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            header: DataHeader{name},
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

impl<T: 'static + Send + Any> ConnectorTrait for Output<T> {
    fn as_any(&self) -> &dyn Any {self}
    fn as_any_mut(&mut self) -> &mut dyn Any {self}
    fn get_header(&self) -> &DataHeader {&self.header}
}