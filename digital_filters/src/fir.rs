use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::any::Any;
use processor_engine::stream_processor::{StreamProcessor, StreamBlock, StreamBlockDyn};
use processor_engine::stream_processor::{StreamingError, StreamingState};
use processor_engine::{create_input, create_output, create_parameter};
use processor_engine::connectors::{Input, Output, Parameter};
use stream_proc_macro::StreamBlockMacro;

#[derive(StreamBlockMacro)]
pub struct FirFilter {
    inputs:     HashMap<&'static str, Box<Input<Vec<f64>>>>,
    outputs:    HashMap<&'static str, Box<Output<Vec<f64>>>>,
    parameters: HashMap<&'static str, Box<Parameter<Vec<f64>>>>,
    state: std::sync::Mutex<StreamingState>,
    lock: std::sync::Mutex<()>,
}

impl FirFilter
{
    pub fn new(coefficients: Vec<f64>) -> Self {
        let mut inputs: HashMap<&'static str, Box<Input<Vec<f64>>>> = HashMap::new();
        let mut outputs: HashMap<&'static str, Box<Output<Vec<f64>>>> = HashMap::new();
        let mut parameters: HashMap<&'static str, Box<Parameter<Vec<f64>>>> = HashMap::new();

        // Define inputs
        let input_vector = create_input!(Vec<f64>, "input_vector", "Input Signal Vector");
        inputs.insert("input_vector", input_vector);

        // Define outputs
        let output_vector = create_output!(Vec<f64>, "output_vector", "Output Signal Vector");
        outputs.insert("output_vector", output_vector);

        // Define parameters
        let coeff_param = create_parameter!(Vec<f64>, "coefficients", "FIR Filter Coefficients", coefficients);
        parameters.insert("coefficients", coeff_param);

        FirFilter {
            inputs,
            outputs,
            parameters,
            state: std::sync::Mutex::new(StreamingState::Null),
            lock: std::sync::Mutex::new(()),
        }
    }

    fn fir_filter(&self, input: &Vec<f64>, coefficients: &Vec<f64>) -> Vec<f64> {
        let filter_len = coefficients.len();
        let input_len = input.len();
        let mut output = vec![0.0; input_len];
        let _lock = self.lock.lock().unwrap();
        for n in 0..input_len {
            let mut acc = 0.0;
            for k in 0..filter_len {
                if n >= k {
                    acc += coefficients[k] * input[n - k];
                }
            }
            output[n] = acc;
        }
        output
    }
}

impl StreamProcessor for FirFilter {
    fn process(&mut self) -> Result<(), processor_engine::stream_processor::StreamingError> {
        let input_vector = self.inputs["input_vector"].recv();

        let coefficients = self.parameters["coefficients"].get_value();

        if input_vector.is_empty() || coefficients.is_empty() {
            return Err(processor_engine::stream_processor::StreamingError::InvalidInput);
        }

        let output_vector = self.fir_filter(&input_vector, &coefficients);

        self.outputs["output_vector"].send(output_vector);

        Ok(())
    }
} 
