use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use data_model::streaming_data::StreamingError;

use crate::stream_processor::StreamProcessor;

pub struct ProcessorEngine {
    processor_map: HashMap<&'static str, Box<dyn StreamProcessor>>,
}

impl ProcessorEngine {
    fn new() -> Self {
        Self { processor_map: HashMap::new() }
    }
    pub fn get() -> &'static Mutex<ProcessorEngine> {
        PROCESSOR_ENGINE.get_or_init(|| Mutex::new(ProcessorEngine::new()))
    }
    pub fn register_processor(&mut self, name: &'static str, processor: Box<dyn StreamProcessor>) -> Result<(), StreamingError> {
        if self.processor_map.contains_key(name) {
            return Err(StreamingError::AlreadyDefined);
        }
        self.processor_map.insert(name, processor);
        Ok(())
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