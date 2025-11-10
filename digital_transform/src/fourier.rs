use std::fmt::{Debug, Display};
use std::collections::HashMap;
use std::ffi::c_char;
use std::sync::mpsc::Sender;
use std::any::Any;
use num_traits::{Float, Zero};
use processor_engine::{create_input, create_output, create_parameter};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor, StreamProcessorStruct};
use processor_engine::stream_processor::{StreamingError, StreamingState};
use processor_engine::connectors::{Input, Output, Parameter};
use processor_engine::connectors::ConnectorsTrait;
use stream_proc_macro::StreamBlockMacro;

use processor_engine::math::{calculus::factorize, complex::Complex};


#[unsafe(no_mangle)]
pub static FFT_PROCESS: StreamProcessorStruct = StreamProcessorStruct {
    name: b"Fast Fourier Transform\0".as_ptr() as *const c_char,
    description: b"Fast Fourier Transform Process Block\0".as_ptr() as *const c_char,
    input_number: 2,
    inputs: &[b"real_input\0".as_ptr() as *const c_char, b"complex_input\0".as_ptr() as *const c_char],
    inputs_type: &[b"Vec<T>, Vec<Complex<T>>\0".as_ptr() as *const c_char],
    output_number: 1,
    outputs: &[b"output\0".as_ptr() as *const c_char],
    outputs_type: &[b"Vec<Complex<T>>\0".as_ptr() as *const c_char],
    parameter_number: 1,
    parameters: &[b"fft_type_input\0".as_ptr() as *const c_char],
    parameters_type: &[b"FftInputType\0".as_ptr() as *const c_char],
};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum FftInputType {
    Real,

}

pub struct Fft<T> {
    size: usize,
    weights: Vec<Complex<T>>,
    factorization: Vec<usize>,
}

impl<T> Fft<T>
where
    T: Into<f64> + From<f64> + Copy + Zero + Float + Debug + Display,
{
    pub fn new(inverse: bool, size: usize) -> Self {
        let mut weights: Vec<Complex<T>> = Vec::with_capacity(size);
        for i in 0..size {
            let angle = if inverse {
                <T as From<f64>>::from(2.0 * std::f64::consts::PI * (i as f64) / (size as f64))
            } else {
                <T as From<f64>>::from(-2.0 * std::f64::consts::PI * (i as f64) / (size as f64))
            };
            weights.push(Complex::new(angle.cos(), angle.sin()));
        }
        let factorization = factorize(size as u64)
            .iter()
            .map(|&x| x as usize)
            .collect();
        Self {
            size,
            weights,
            factorization: factorization,
        }
    }

    fn fft_process(&self,
        input: &Vec<Complex<T>>,
        size: usize,
        index_factor: usize,
        start: usize,
        step: usize) -> Vec<Complex<T>>
    where
        T: Debug + Display + Float,
    {
        if size == 1 {
            return input.clone();
        }
        let mut output : Vec<Complex<T>> = vec![Complex::new(T::zero(), T::zero()); size];

        if size == 2 {
            output[0] = input[0].clone() + input[1].clone();
            output[1] = input[0].clone() - input[1].clone();
            
            return output;
        }
        let chunk_number = self.factorization[index_factor];
        let chunk_size = size / chunk_number;
        let mut chunks : Vec<Vec<Complex<T>>> = vec![vec![Complex::new(T::zero(), T::zero()); chunk_size]; chunk_number];
        for i in 0..chunk_number {
            let chunk_fft = self.fft_process(
                input,
                chunk_size,
                index_factor + 1,
                start + i * step,
                step * chunk_number,
            );
            chunks[i] = chunk_fft;
        }
        for k in 0..size {
            for i in 0..chunk_number as usize {
                let index_sel = k % chunk_size as usize;
                output[k] += chunks[i][index_sel] * self.weights[index_sel];
            }
        }
        output
    }

    pub fn fft_real(&self, input: &Vec<T>) -> Vec<Complex<T>>
    where
        T: Into<T> + Copy + Float + Debug + Display,
    {
        let mut input: Vec<Complex<T>> = input.iter().map(|&x| Complex::new(x, T::zero())).collect();
        if input.len() < self.size {
            let mut padded_input = input.clone();
            for _ in input.len()..self.size {
                padded_input.push(Complex::new(T::zero(), T::zero()));
            }
            input = padded_input;
        }
        self.fft_process(&input, self.size, 0, 0, 1)
    }
    pub fn fft_complex(&self, input: &Vec<Complex<T>>) -> Vec<Complex<T>> 
    {
        if input.len() < self.size {
            let mut padded_input = input.clone();
            for _ in input.len()..self.size {
                padded_input.push(Complex::new(T::zero(), T::zero()));
            }
            return self.fft_process(&padded_input, self.size, 0, 0, 1);
        }
        self.fft_process(input, self.size, 0, 0, 1)
    }
}

#[derive(StreamBlockMacro)]
pub struct FftProcess<T: 'static + Send + Clone> {
    inputs:     HashMap<&'static str, Box<dyn ConnectorsTrait>>,
    outputs:    HashMap<&'static str, Box<Output<Vec<Complex<T>>>>>,
    parameters: HashMap<&'static str, Box<Parameter<FftInputType>>>,
    state: std::sync::Mutex<StreamingState>,
    lock: std::sync::Mutex<()>,
    fft_planner: Fft<T>,
}

impl <T> FftProcess<T>
where
    T: 'static 
    + Send 
    + Clone 
    + Into<f64> 
    + From<f64>
    + Float
    + Debug
    + Display,
{
    pub fn new(size: usize, inverse: bool) -> Self {
        let fft_planner = Fft::<T>::new(inverse, size);
        let mut inputs: HashMap<&'static str, Box<dyn ConnectorsTrait>> = HashMap::new();
        inputs.insert(
            "input",
            create_input!(Vec<Complex<T>>, "complex_input", "Complex input Signal"),
        );
        inputs.insert(
            "input_real",
            create_input!(Vec<T>, "real_input", "Real input Signal"),
        );
        let mut outputs: HashMap<&'static str, Box<Output<Vec<Complex<T>>>>> = HashMap::new();
        outputs.insert(
            "output",
            create_output!(Vec<Complex<T>>, "output", "Output Complex Signal"),
        );
        let mut parameters: HashMap<&'static str, Box<Parameter<FftInputType>>> = HashMap::new();
        parameters.insert(
            "fft_type_input",
            create_parameter!(FftInputType, "fft_type_input", "FFT Input Type", FftInputType::Real),
        );
        Self {
            inputs,
            outputs,
            parameters,
            state: std::sync::Mutex::new(StreamingState::Stopped),
            lock: std::sync::Mutex::new(()),
            fft_planner,
        }
    }
}

impl<T> StreamProcessor for FftProcess<T> 
where
    T: 'static 
        + Send 
        + Clone 
        + Into<f64> 
        + From<f64>
        + Float
        + Debug
        + Display,
{
    fn process(&mut self) -> Result<(), StreamingError> {
        let fft_input = self.parameters.get("fft_type_input")
            .expect("Missing parameter")
            .as_any()
            .downcast_ref::<Parameter<FftInputType>>()
            .unwrap()
            .get_value()
            .clone();
        let _guard = self.lock.lock().unwrap();
        if fft_input == FftInputType::Real {
            let input_real = self.inputs.get("real_input")
                .expect("Missing input")
                .as_any()
                .downcast_ref::<Input<Vec<T>>>()
                .unwrap()
                .recv();
            let _guard = self.state.lock().unwrap();
            let fft_result = self.fft_planner.fft_real(&input_real);
            self.outputs.get_mut("output")
                .expect("Missing output")
                .send(fft_result);
        } else {
            let input_complex = self.inputs.get("input")
                .expect("Missing input")
                .as_any()
                .downcast_ref::<Input<Vec<Complex<T>>>>()
                .unwrap()
                .recv();
            let _guard = self.state.lock().unwrap();
            let fft_result = self.fft_planner.fft_complex(&input_complex);
            self.outputs.get_mut("output")
                .expect("Missing output")
                .send(fft_result);
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft() {
        let size = 1024;
        let planner = Fft::<f64>::new(false, size);
        let signal: Vec<Complex<f64>> = (0..size)
            .map(|x| Complex::new(x as f64, 0.0))
            .collect();
        for _ in 0..1000 {
            let fft_result = planner.fft_complex(&signal);
            for (i, value) in fft_result.iter().enumerate() {
                println!("FFT Result[{}]: {:?}", i, value);
            }
        }
    }
}