use std::collections::HashMap;
use std::ops::{Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign};
use std::sync::mpsc::Sender;
use std::any::Any;

use num_traits::Float;
use kappa_library::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use kappa_library::stream_processor::{StreamingError, StreamingState};
use kappa_library::{create_input, create_output, create_parameter};
use kappa_library::connectors::{Input, Output, Parameter};
use kappa_library::connectors::ConnectorsTrait;
use stream_proc_macro::StreamBlockMacro;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CfarFilterType {
    CellAveraging,
    GreatestOf,
    LowestOf,
}

fn cfar_filter<T>(input: &Vec<T>, 
                      filter_logic: CfarFilterType, 
                      threshold: T,
                      cfar_guard: i32,
                      cfar_cell: i32) -> (Vec<bool>, Vec<T>)
where
    T: Into<f64> 
    + Copy
    + Float
    + Add<Output = T>
    + AddAssign
    + Sub<Output = T>
    + SubAssign
    + Mul<Output = T>
    + MulAssign
    + Div
    + DivAssign
    + From<f64>,
{
    let len = input.len();
    let mut output = vec![false; len];
    let mut filter_output = vec![T::zero(); len];
    let mut sum_buffer: Vec<T> = vec![T::zero(); len];

    for i in 0..cfar_cell as usize {
        sum_buffer[0] += input[i];
    }
    let cfar_cell_index = cfar_cell as usize;
    let cfar_guard_index = cfar_guard as usize;
    let len_i32 = len as i32;
    for i in 1..len + cfar_guard_index as usize {
        if (i as i32) < (len as i32) - cfar_cell {
            sum_buffer[i] = sum_buffer[i-1] 
                            + input[i + cfar_cell as usize - 1]
                            - input[i - 1];
        }
        if (i as i32) >= cfar_guard {
            let index_out = i - cfar_guard_index;
            let mut sum: T;
            if (i as i32) <= cfar_cell + 2*cfar_guard {
                sum = sum_buffer[i];
            } else if (i as i32) > len_i32 - cfar_cell {
                sum = sum_buffer[i - cfar_cell_index - 2*cfar_guard_index - 1];
            } else {
                let sum_early = sum_buffer[i - cfar_cell_index - 2*cfar_guard_index - 1];
                let sum_late = sum_buffer[i];
                match filter_logic {
                    CfarFilterType::CellAveraging => {
                        sum = (sum_early + sum_late)/2.0.into();
                    }
                    CfarFilterType::GreatestOf => {
                        sum = if sum_early.into() > sum_late.into() { sum_early } else { sum_late };
                    }
                    CfarFilterType::LowestOf => {
                        sum = if sum_early.into() < sum_late.into() { sum_early } else { sum_late };
                    }
                }
            }
            sum = sum / (cfar_cell as f64).into();
            let threshold_value: T = threshold + sum;
            if input[index_out].into() > threshold_value.into() {
                output[index_out] = true;
                filter_output[index_out] = input[index_out];
            } else {
                output[index_out] = false;
                filter_output[index_out] = T::zero();
            }
        }
    }
    (output, filter_output)
}

#[derive(StreamBlockMacro)]
pub struct CfarProcess<T: 'static + Send + Clone> {
    inputs:     HashMap<&'static str, Box<Input<Vec<T>>>>,
    outputs:    HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    parameters: HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    state: std::sync::Mutex<StreamingState>,
    lock: std::sync::Mutex<()>,
}

impl<T> CfarProcess<T>
where
    T: 'static 
    + Send 
    + Clone 
    + Into<f64> 
    + From<f64>
    + Float
    + Add<Output = T>
    + AddAssign
    + Sub<Output = T>
    + SubAssign
    + Mul<Output = T>
    + MulAssign
    + Div
    + DivAssign,
{
    pub fn new() -> Self {
        let mut inputs: HashMap<&'static str, Box<Input<Vec<T>>>> = HashMap::new();
        let mut outputs: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        let mut parameters: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        // Define inputs
        inputs.insert("input", create_input!(Vec<T>, "input", "Input Signal Vector"));
        let _a = &inputs["input"];
        // Define outputs
        outputs.insert("filter_signal", create_output!(Vec<T>, "filter_signal", "Output Signal Vector"));
        outputs.insert("pass_vector", create_output!(Vec<bool>, "pass_vector", "CFAR Pass Vector"));
        // Define parameters
        parameters.insert("cfar_type", create_parameter!(CfarFilterType, "cfar_type", "CFAR Type", CfarFilterType::CellAveraging));
        parameters.insert("cfar_threshold", create_parameter!(T, "cfar_threshold", "CFAR Threshold", T::one() * 10.0.into()));
        parameters.insert("cfar_cell", create_parameter!(i32, "cfar_cell", "CFAR Cell", 10));
        parameters.insert("cfar_guard", create_parameter!(i32, "cfar_guard", "CFAR Guard", 3));
        Self {
            inputs,
            outputs,
            parameters,
            state: std::sync::Mutex::new(StreamingState::Null),
            lock: std::sync::Mutex::new(()),
        }
    }
}
impl<T> StreamProcessor for CfarProcess<T>
where
    T: 'static 
        + Send 
        + Clone 
        + Copy
        + Into<f64> 
        + From<f64>
        + PartialOrd
        + Float
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + SubAssign
        + Mul<Output = T>
        + MulAssign
        + Div
        + DivAssign,
{
    fn process(&mut self) -> Result<(), StreamingError> {
        let _guard = self.lock.lock().unwrap();
        let input_signal = self.inputs["input"].recv();
        let cfar_type = self.parameters["cfar_type"]
                .as_any()
                .downcast_ref::<Parameter<CfarFilterType>>()
                .unwrap()
                .get_value()
                .clone();
        let cfar_threshold = self.parameters["cfar_threshold"]
                .as_any()
                .downcast_ref::<Parameter<T>>()
                .unwrap()
                .get_value()
                .clone();
        let cfar_cell = self.parameters["cfar_cell"]
                .as_any()
                .downcast_ref::<Parameter<i32>>()
                .unwrap()
                .get_value()
                .clone();
        let cfar_guard = self.parameters["cfar_guard"]
                .as_any()
                .downcast_ref::<Parameter<i32>>()
                .unwrap()
                .get_value()
                .clone();
        let (pass_vector, filtered_values) = cfar_filter(&input_signal, cfar_type, cfar_threshold, cfar_guard, cfar_cell);
        
        self.outputs.get_mut("filter_signal")
            .expect("Missing output")
            .as_any_mut()
            .downcast_mut::<Output<Vec<T>>>()
            .unwrap()
            .send(filtered_values);
        self.outputs.get_mut("pass_vector")
            .expect("Missing output")
            .as_any_mut()
            .downcast_mut::<Output<Vec<bool>>>()
            .unwrap()
            .send(pass_vector);
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cfar_filter() {
        let mut cfar_process = CfarProcess::<f64>::new();
        let cfar_type = CfarFilterType::CellAveraging;
        let cfar_threshold = 20.0;
        let cfar_cell = 5;
        let cfar_guard = 2;
        cfar_process.set_parameter_value("cfar_type", cfar_type).unwrap();
        cfar_process.set_parameter_value("cfar_threshold", cfar_threshold).unwrap();
        cfar_process.set_parameter_value("cfar_cell", cfar_cell).unwrap();
        cfar_process.set_parameter_value("cfar_guard", cfar_guard).unwrap();
        let (tx_data, rx_data) = std::sync::mpsc::channel();
        let (tx_pass, rx_pass) = std::sync::mpsc::channel();
        cfar_process.connect::<Vec<f64>>("filter_signal", tx_data);
        cfar_process.connect::<Vec<bool>>("pass_vector", tx_pass);
        let input_signal: Vec<f64> = (0..65536).map(|_| rand::random::<f64>() * 100.0).collect();
        for _ in 0..1000 {
            cfar_process.inputs.get_mut("input").unwrap().send(input_signal.clone());
            cfar_process.process().unwrap();
            let _filtered_output = rx_data.recv().unwrap();
            let _pass_vector = rx_pass.recv().unwrap();
        }
    }
}
