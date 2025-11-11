use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::any::Any;
use processor_engine::stream_processor::{StreamProcessor, StreamBlock, StreamBlockDyn};
use processor_engine::stream_processor::{StreamingError, StreamingState};
use processor_engine::{create_input, create_output, create_parameter};
use processor_engine::connectors::{Input, Output, Parameter};
use stream_proc_macro::StreamBlockMacro;

#[derive(StreamBlockMacro)]
pub struct IirFilter {
    inputs:     HashMap<&'static str, Box<Input<Vec<f64>>>>,
    outputs:    HashMap<&'static str, Box<Output<Vec<f64>>>>,
    parameters: HashMap<&'static str, Box<Parameter<Vec<f64>>>>,
    state: std::sync::Mutex<StreamingState>,
    lock: std::sync::Mutex<()>,
}

impl IirFilter
{
    pub fn new(n_coefficients: Vec<f64>, d_coefficients: Vec<f64>) -> Self {
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
        let n_coeff_param = create_parameter!(Vec<f64>, "n_coefficients", "IIR Numerator Filter Coefficients", n_coefficients);
        parameters.insert("n_coefficients", n_coeff_param);
        let d_coeff_param = create_parameter!(Vec<f64>, "d_coefficients", "IIR Denominator Filter Coefficients", d_coefficients);
        parameters.insert("d_coefficients", d_coeff_param);
        IirFilter {
            inputs,
            outputs,
            parameters,
            state: std::sync::Mutex::new(StreamingState::Null),
            lock: std::sync::Mutex::new(()),
        }
    }

    fn iir_filter(&self, input: &Vec<f64>, n_coefficients: &Vec<f64>, d_coefficients: &Vec<f64>) -> Vec<f64> {
        let filter_len = n_coefficients.len();
        let input_len = input.len();
        let mut output = vec![0.0; input_len];
        let mut state = vec![0.0; filter_len - 1];
        let _lock = self.lock.lock().unwrap();
        for n in 0..input_len {
            let mut acc = n_coefficients[0] * input[n];
            for k in 1..filter_len {
                if n >= k {
                    acc += n_coefficients[k] * input[n - k];
                    acc -= d_coefficients[k] * output[n - k];
                } else {
                    acc += n_coefficients[k] * 0.0;
                    acc -= d_coefficients[k] * state[filter_len - 1 - (k - n)];
                }
            }
            output[n] = acc;
            if n < filter_len - 1 {
                state[n] = output[n];
            }
        }
        output
    }
}

impl StreamProcessor for IirFilter {
    fn process(&mut self) -> Result<(), processor_engine::stream_processor::StreamingError> {
        let input_vector = self.inputs["input_vector"].recv();

        let n_coefficients = self.parameters["n_coefficients"].get_value();
        let d_coefficients = self.parameters["d_coefficients"].get_value();

        if input_vector.is_empty() || n_coefficients.is_empty() || d_coefficients.is_empty() {
            return Err(processor_engine::stream_processor::StreamingError::InvalidInput);
        }

        let output_vector = self.iir_filter(&input_vector, &n_coefficients, &d_coefficients);

        self.outputs["output_vector"].send(output_vector);

        Ok(())
    }
} 
