use std::any::Any;
use std::sync::mpsc::{SyncSender, Receiver};
use data_model::memory_manager::DataHeader;
use data_model::streaming_data::StreamingError;

pub trait ConnectorTrait: Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn get_header(&self) -> &DataHeader;
}

pub struct Input<T: 'static + Send + Any + Clone> {
    pub header: DataHeader,
    pub sender: SyncSender<T>,
    receiver: Receiver<T>,
}

impl<T> Input<T> 
where T: 'static + Send + Any + Clone
{
    pub fn new(name: &'static str) -> Self{
        let (sender, receiver) = std::sync::mpsc::sync_channel(50);
        Self {
            header: DataHeader{name},
            sender,
            receiver,
        }
    }
    pub fn send(&mut self, data: T) -> Result<(), StreamingError>{
        let ret = self.sender.send(data);
        if ret.is_ok() {
            Ok(())
        } else {
            Err(StreamingError::SendDataError)
        }
    }
    pub fn recv(&mut self) -> Result<T, StreamingError>{
        let ret = self.receiver.recv();
        if ret.is_ok() {
            Ok(ret.unwrap())
        } else {
            Err(StreamingError::ReceiveDataError)
        }
    }
}
impl<T: 'static + Send + Any + Clone> ConnectorTrait for Input<T> {
    fn as_any(&self) -> &dyn Any {self}
    fn as_any_mut(&mut self) -> &mut dyn Any {self}
    fn get_header(&self) -> &DataHeader {&self.header}
}

pub struct Output<T: 'static + Send + Clone> {
    pub header: DataHeader,
    pub senders: Vec<SyncSender<T>>,
}

impl<T: 'static + Send + Any + Clone> Output<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            header: DataHeader{name},
            senders: Vec::new(),
        }
    }
    pub fn connect(&mut self, sender: SyncSender<T>) {
        self.senders.push(sender);
    }
    pub fn send(&self, data: T) -> Result<(), StreamingError>{
        for s in &self.senders {
            let res = s.send(data.clone());
            match res {
                Ok(_) => {continue;},
                Err(_) => {return Err(StreamingError::SendDataError);}
            }
        }
        Ok(())
    }
}

impl<T: 'static + Send + Any + Clone> ConnectorTrait for Output<T> {
    fn as_any(&self) -> &dyn Any {self}
    fn as_any_mut(&mut self) -> &mut dyn Any {self}
    fn get_header(&self) -> &DataHeader {&self.header}
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_input() {
        let mut test_input = Input::<f64>::new("test_input");
        test_input.send(0.8);
        let mut received = test_input.recv().unwrap();
        assert_eq!(received, 0.8);
        test_input.send(1.0).unwrap();
        test_input.send(2.0).unwrap();
        test_input.send(3.0).unwrap();
        received = test_input.recv().unwrap();
        assert_eq!(received, 1.0);
        received = test_input.recv().unwrap();
        assert_eq!(received, 2.0);
        received = test_input.recv().unwrap();
        assert_eq!(received, 3.0);
    }
    #[test]
    fn test_output() {
        let mut test_output = Output::<u32>::new("test_output");
        let mut test_input = Input::<u32>::new("test_input");
        test_output.connect(test_input.sender.clone());
        test_output.send(1).unwrap();
        test_output.send(2).unwrap();
        let mut recv = test_input.recv().unwrap();
        eprintln!("Received {}", recv);
        assert_eq!(recv, 1);
        recv = test_input.recv().unwrap();
        assert_eq!(recv, 2);

    }
}