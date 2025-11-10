use std::collections::HashMap;
use std::ffi::c_char;
use std::sync::mpsc::Sender;
use std::any::Any;
use chrono::{DateTime, Utc};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use processor_engine::stream_processor::{StreamingError, StreamingState, StreamProcessorStruct};
use utils::math::numbers::factorial;
use processor_engine::{create_input, create_output, create_parameter};
use processor_engine::connectors::{Input, Output, Parameter};
use processor_engine::connectors::ConnectorsTrait;
use stream_proc_macro::StreamBlockMacro;

#[derive(Clone)]
pub struct TimeTaggedSample<T: Send + Clone + 'static> {
    pub time: DateTime<Utc>,
    pub value: T,
}

pub struct EkfFilter<T> {
    pub filter_order: usize,
    matrix_state_transition: Vec<Vec<T>>, // F matrix
    estimated_state: Vec<T>, // x_hat
    filter_covariance: Vec<Vec<T>>, // P matrix
}

impl<T> EkfFilter<T> 
where T: Clone 
        + Copy
        + Send
        + num_traits::Zero
        + num_traits::One
        + std::ops::Add<Output = T>
        + std::ops::AddAssign
        + std::ops::Mul<<T as std::ops::Sub>::Output, Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + std::convert::From<f64>
        + std::fmt::Debug
{
    pub fn new(initial_state: Vec<T>, sampling_time: f64) -> Self {
        let filter_order = initial_state.len();
        // Define state transition matrix
        let mut matrix_state_transition = vec![vec![T::zero(); filter_order]; filter_order];
        for i in 0..filter_order {
            matrix_state_transition[i][i] = T::one();
            for n in i+1..filter_order {
                let binding = sampling_time.powi((n - i) as i32) / (factorial((n - i) as u64) as f64);
                matrix_state_transition[i][n] = binding.into();
            }
        }
        Self {
            filter_order,
            matrix_state_transition,
            estimated_state: initial_state,
            filter_covariance: vec![vec![T::zero(); filter_order]; filter_order],
        }
    }

    fn ekf_filter(&mut self, input: &Option<TimeTaggedSample<T>>, process_noise: &Vec<Vec<T>>, measurement_noise: &T) -> TimeTaggedSample<T> {
        // Predicted State Estimate
        let mut predicted_state = vec![T::zero(); self.filter_order];
        for i in 0..self.filter_order {
            for j in i..self.filter_order {
                predicted_state[i] += self.matrix_state_transition[i][j] * self.estimated_state[j];
            }
        }
        // Predicted Estimate Covariance
        let mut predicted_covariance = vec![vec![T::zero(); self.filter_order]; self.filter_order];
        for i in 0..self.filter_order {
            for j in 0..self.filter_order {
                for k in 0..self.filter_order {
                    predicted_covariance[i][j] = predicted_covariance[i][j]
                        + <T as Into<T>>::into(self.matrix_state_transition[i][k])
                        * <T as Into<T>>::into(self.matrix_state_transition[j][k])
                        * <T as Into<T>>::into(self.filter_covariance[k][k]);
                }
                predicted_covariance[i][j] = predicted_covariance[i][j] + process_noise[i][j];
            }
        }
        // Residual Covariance
        let residual_covariance = predicted_covariance[0][0] + *measurement_noise;
        // Kalman Gain
        let mut kalman_gain = vec![T::zero(); self.filter_order];
        for i in 0..self.filter_order {
            kalman_gain[i] = predicted_covariance[i][0] / residual_covariance;
        }
        // Updated Estimate Covariance
        for i in 0..self.filter_order {
            for j in 0..self.filter_order {
                self.filter_covariance[i][j] = predicted_covariance[i][j]
                    - kalman_gain[i] * predicted_covariance[0][j];
            }
        }
        // Measurement Residual
        if let Some(new_value) = input {
            let measurement_residual = new_value.value - predicted_state[0];
            // Updated State Estimate
            for i in 0..self.filter_order {
                self.estimated_state[i] = predicted_state[i] + kalman_gain[i] * measurement_residual;
            }
        } else {
            // No measurement update
            for i in 0..self.filter_order {
                self.estimated_state[i] = predicted_state[i];
            }
        }
        let output = TimeTaggedSample::<T> {
            time: Utc::now(),
            value: self.estimated_state[0],
        };
        output
    }
}

#[derive(StreamBlockMacro)]
pub struct EkfProcess<T: 'static + Send + Clone> {
    inputs:     HashMap<&'static str, Box<Input<Option<TimeTaggedSample<T>>>>>,
    outputs:    HashMap<&'static str, Box<Output<TimeTaggedSample<T>>>>,
    parameters: HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    state: std::sync::Mutex<StreamingState>,
    lock: std::sync::Mutex<()>,
    filter: EkfFilter<T>,
}

#[unsafe(no_mangle)]
pub static EKF_FILTER: StreamProcessorStruct = StreamProcessorStruct {
    name: b"EkfFilter\0".as_ptr() as *const c_char,
    description: b"Extended Kalman Filter Process Block\0".as_ptr() as *const c_char,
    input_number: 1,
    inputs: &[b"input\0".as_ptr() as *const c_char],
    inputs_type: &[b"Option<TimeTaggedSample<T>>\0".as_ptr() as *const c_char],
    output_number: 1,
    outputs: &[b"output\0".as_ptr() as *const c_char],
    outputs_type: &[b"TimeTaggedSample<T>\0".as_ptr() as *const c_char],
    parameter_number: 2,
    parameters: &[b"process_noise\0".as_ptr() as *const c_char, b"measurement_noise\0".as_ptr() as *const c_char],
    parameters_type: &[b"Vec<Vec<T>>\0".as_ptr() as *const c_char, b"T\0".as_ptr() as *const c_char],
};

impl<T> EkfProcess<T>
where T: 'static
        + Clone 
        + Copy
        + Send
        + PartialOrd
        + num_traits::Zero
        + num_traits::One
        + std::ops::Add<Output = T>
        + std::ops::AddAssign
        + std::ops::Mul<<T as std::ops::Sub>::Output, Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + std::convert::From<f64>
        + std::fmt::Debug
{
    pub fn new(initial_state: Vec<T>, sampling_time: f64) -> Self {
        let mut inputs: HashMap<&'static str, Box<Input<Option<TimeTaggedSample<T>>>>> = HashMap::new();
        let mut outputs: HashMap<&'static str, Box<Output<TimeTaggedSample<T>>>> = HashMap::new();
        let mut parameters: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        // Define inputs
        let input = create_input!(Option<TimeTaggedSample<T>>, "input", "Input Signal Vector");
        inputs.insert("input", input);
        let _a = &inputs["input"];
        // Define outputs
        let output = create_output!(TimeTaggedSample<T>, "output", "Output Signal Vector");
        outputs.insert("output", output);
        // Define parameters
        let filter_order = initial_state.len();
        parameters.insert("process_noise", create_parameter!(Vec<Vec<T>>, "process_noise", "Process Noise Covariance", vec![vec![T::zero(); filter_order]; filter_order]));
        parameters.insert("measurement_noise", create_parameter!(T, "measurement_noise", "Measurement Noise Covariance", T::zero()));
        Self {
            inputs,
            outputs,
            parameters,
            state: std::sync::Mutex::new(StreamingState::Null),
            lock: std::sync::Mutex::new(()),
            filter: EkfFilter::new(initial_state, sampling_time),
        }
    }
}

impl<T> StreamProcessor for EkfProcess<T> 
where T: 'static 
        + Clone 
        + Copy
        + Send
        + num_traits::Zero
        + num_traits::One
        + std::ops::Add<Output = T>
        + std::ops::AddAssign
        + std::ops::Mul<<T as std::ops::Sub>::Output, Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + std::convert::From<f64>
        + PartialOrd
        + std::fmt::Debug
{
    fn process(&mut self) -> Result<(), StreamingError> {
        let input = self.inputs["input"].recv();
        let process_noise: Vec<Vec<T>> = self.get_parameter_value("process_noise")?;
        let measurement_noise: T = self.get_parameter_value("measurement_noise")?;

        if process_noise.len() != self.filter.filter_order || process_noise[0].len() != self.filter.filter_order {
            return Err(StreamingError::InvalidParameter);
        }
        let _guard = self.lock.lock().unwrap();
        let output = self.filter.ekf_filter(&input, &process_noise, &measurement_noise);

        self.outputs["output"].send(output);
            
        Ok(())
    }
} 

#[cfg(test)]
mod tests {
    use std::fs;
    use std::slice;
    use super::*;
    #[test]
    fn test_ekf_filter() {
        let mut ekf = EkfFilter::<f64>::new(vec![0.0, 0.0, 0.0, 0.0], 1.0);
        let process_noise = vec![vec![1e-5, 0.0], vec![0.0, 1e-5]];
        let measurement_noise = 1e-2;
        let input_sample = Some(TimeTaggedSample {
            time: Utc::now(),
            value: 1.0,
        });
        let output = ekf.ekf_filter(&input_sample, &process_noise, &measurement_noise);
        assert!(output.value.is_finite());
    }

    #[test]
    fn test_ekf_process() {
        let file_path = "test/ekf_input.bin";
        use std::path::Path;
        let init_state = vec![0.0, 0.0, 0.0, 0.0];
        if !Path::new(file_path).exists() {
            let time_factor: f64 = 0.1;
            let number_of_samples = 10000;
            let mut data: Vec<f64> = Vec::new();
            let mut current_state = init_state.clone();
            for _ in 0..number_of_samples {
                current_state[0] += current_state[1] * time_factor
                                 + current_state[2] * (time_factor.powi(2)) / 2.0
                                 + current_state[3] * (time_factor.powi(3)) / 6.0
                                 + (rand::random::<f64>() - 0.5) * 5.0;
                current_state[1] += current_state[2] * time_factor
                                 + current_state[3] * (time_factor.powi(2)) / 2.0
                                 + (rand::random::<f64>() - 0.5) * 1.0;
                current_state[2] += current_state[3] * time_factor 
                                 + (rand::random::<f64>() - 0.5) * 0.5;
                current_state[3] += (rand::random::<f64>() - 0.5) * 0.1;
                // Simple motion model: constant velocity
                data.push(current_state[0]);
            } 
            
            let total_bytes = data.len() * 8; 
            let byte_slice: &[u8] = unsafe {
                let ptr = data.as_ptr();
                let u8_ptr = ptr as *const u8;
                std::slice::from_raw_parts(u8_ptr, total_bytes)
            };
            fs::write(file_path, byte_slice);
        }
        let mut byte_data: Vec<u8> = fs::read(file_path).unwrap();
        let total_bytes = byte_data.len();
        if total_bytes % 8 != 0 {
            panic!("Warning: Byte count ({}) is not a multiple of 8 (size of f64). Truncating.", total_bytes);
        }

        let f64_count = total_bytes / 8;
        let data_input = unsafe {
            let ptr = byte_data.as_mut_ptr();
            let f64_ptr = ptr as *mut f64;
            let f64_slice = slice::from_raw_parts_mut(f64_ptr, f64_count);
            Vec::from_raw_parts(f64_ptr, f64_count, f64_count)
        };
        std::mem::forget(byte_data);

        let mut ekf_process = EkfProcess::<f64>::new(init_state, 1.0);
        let process_noise = vec![vec![1e-5; 4]; 4];
        let measurement_noise = 1e-2;
        ekf_process.set_parameter_value("process_noise", process_noise).unwrap();
        ekf_process.set_parameter_value("measurement_noise", measurement_noise).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        ekf_process.connect::<TimeTaggedSample<f64>>("output", tx);
        let mut result_vec : Vec<TimeTaggedSample<f64>> = Vec::new();
        for &value in data_input.iter() {
            let input_sample = Some(TimeTaggedSample {
                time: Utc::now(),
                value,
            });
            ekf_process.get_input_channel("input").unwrap().send(input_sample);
            ekf_process.process().unwrap();
            let output = rx.recv().unwrap();
            assert!(output.value.is_finite());
            result_vec.push(output);
        }
        let total_bytes = result_vec.len() * 8;
        let f64_count = result_vec.len();
        let byte_slice: &[u8] = unsafe {
            let ptr = result_vec.as_ptr();
            let u8_ptr = ptr as *const u8;
            std::slice::from_raw_parts(u8_ptr, total_bytes)
        };
        fs::write("test/ekf_output.bin", byte_slice);
    }

    #[test]
    fn test_performance() {
        let init_state = vec![0.0; 6];
        let mut ekf = EkfFilter::<f64>::new(init_state, 0.1);
        let process_noise = vec![vec![1e-5; 6]; 6];
        let measurement_noise = 1e-2;
        let start = Utc::now();
        for i in 0..1000000 {
            let input_sample = Some(TimeTaggedSample {
                time: Utc::now(),
                value: i as f64,
            });
            let output = ekf.ekf_filter(&input_sample, &process_noise, &measurement_noise);
            assert!(output.value.is_finite());
        }
        let duration = Utc::now() - start;
        println!("Performance test took: {:?}", duration);
    }

    #[test]
    fn test_performance_processor_engine() {
        let init_state = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut ekf_process = EkfProcess::<f64>::new(init_state, 0.1);
        let process_noise = vec![vec![1e-5; 6]; 6];
        let measurement_noise = 1e-2;
        ekf_process.set_parameter_value("process_noise", process_noise).unwrap();
        ekf_process.set_parameter_value("measurement_noise", measurement_noise).unwrap();
        let start = Utc::now();
        let (tx, rx) = std::sync::mpsc::channel();
        ekf_process.connect::<TimeTaggedSample<f64>>("output", tx);
        for i in 0..1 {
            let input_sample = Some(TimeTaggedSample {
                time: Utc::now(),
                value: i as f64,
            });
            ekf_process.get_input_channel("input").unwrap().send(input_sample);
            ekf_process.process().unwrap();
            let _output = rx.recv();
        }
        let duration = Utc::now() - start;
        println!("Processor engine performance test took: {:?}", duration);
    }
}