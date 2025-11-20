use std::collections::HashMap;
use std::any::Any;
use std::fmt::{Display, Debug};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use num_traits::{AsPrimitive, Float, Zero};
use serde::Serialize;
use stream_proc_macro::{StreamBlockMacro};
use data_model::streaming_data::{StreamingError, StreamingState};
use data_model::memory_manager::{DataTrait, MemoryManager, Parameter, State, Statics, StaticsTrait};
use processor_engine::stream_processor::{StreamBlock, StreamBlockDyn, StreamProcessor};
use processor_engine::connectors::{ConnectorTrait, Input, Output};
use processor_engine::engine::ProcessorEngine;
use utils::math::{numbers::factorize, complex::Complex};



#[derive(Debug, Clone, PartialEq, PartialOrd, Copy, Serialize)]
pub enum FftInputType {
    Real,
    Complex,
}

impl Display for FftInputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

unsafe impl Send for FftInputType {}
pub struct Fft<T> {
    size: usize,
    weights: Vec<Complex<T>>,
    factorization: Vec<usize>,
}

impl<T> Fft<T>
where
    T: AsPrimitive<f64> + Copy + Zero + Float + Display + Debug, f64: AsPrimitive<T>,
{
    pub fn new(inverse: bool, size: usize) -> Self {
        let mut weights: Vec<Complex<T>> = Vec::with_capacity(size);
        for i in 0..size {
            let angle: T = if inverse {
                (2.0 * std::f64::consts::PI * (i as f64) / (size as f64)).as_()
            } else {
                (-2.0 * std::f64::consts::PI * (i as f64) / (size as f64)).as_()
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
        T: Display + Float,
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
        T: Into<T> + Copy + Float + Display,
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
    inputs:      HashMap<&'static str, Box<dyn ConnectorTrait>>,
    outputs:     HashMap<&'static str, Box<dyn ConnectorTrait>>,
    parameters:  HashMap<&'static str, Box<dyn DataTrait>>,
    statics:     HashMap<&'static str, Box<dyn StaticsTrait>>,
    state:       HashMap<&'static str, Box<dyn StaticsTrait>>,
    name:        &'static str,
    proc_state:  Mutex<StreamingState>,
    lock:        Mutex<()>,
    fft_planner: Fft<T>,
}

impl <T> FftProcess<T>
where
    T: 'static 
    + Send 
    + Clone 
    + AsPrimitive<f64>
    + Float
    + Display
    + Debug,  f64: AsPrimitive<T>,
{
    pub fn new(name: &'static str) -> Self {
        let fft_planner = Fft::<T>::new(false, 2);
        let mut ret= Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            parameters: HashMap::new(),
            statics: HashMap::new(),
            state: HashMap::new(),
            name: name,
            proc_state: Mutex::new(StreamingState::Null),
            lock: Mutex::new(()),
            fft_planner,
        };
        ret.new_input::<Vec<Complex<T>>>("complex_input").unwrap();
        ret.new_input::<Vec<T>>("real_input").unwrap();
        ret.new_output::<Vec<Complex<T>>>("output").unwrap();
        ret.new_parameter::<FftInputType>("fft_type_input", FftInputType::Real, None).unwrap();
        ret.new_statics::<u32>("fft_size", 2).unwrap();
        ret.new_statics::<bool>("inverse", false).unwrap();
        ret
    }
}

impl<T> StreamProcessor for FftProcess<T> 
where
    T: 'static 
        + Send 
        + Clone 
        + AsPrimitive<f64>
        + Float
        + Display
        + Debug,  f64: AsPrimitive<T>,
{
    fn init(&mut self) -> Result<(), StreamingError > {
        if self.check_state(StreamingState::Running) {
            return Err(StreamingError::InvalidStateTransition)
        }
        if !self.is_initialized() {
            return Err(StreamingError::InvalidStatics)
        }
        if self.check_state(StreamingState::Null) {
            let fft_size: usize = self.get_statics::<u32>("fft_size").expect("").get_value().clone() as usize;
            let inverse: bool = self.get_statics::<bool>("inverse").expect("").get_value().clone();
            
            self.fft_planner = Fft::<T>::new(inverse, fft_size);
        }
        self.set_state(StreamingState::Initial);
        Ok(())

    }
    fn process(&mut self) -> Result<(), StreamingError> {
        let fft_input = self.get_parameter::<FftInputType>("fft_type_input").expect("").get_value().clone();
        let fft_result: Vec<Complex<T>>;
        if fft_input == FftInputType::Real {
            let input_real = self.recv_input::<Vec::<T>>("real_input");
            let _guard = self.proc_state.lock().unwrap();
            fft_result = self.fft_planner.fft_real(input_real.unwrap().as_ref());
        } else {
            let input_complex = self.recv_input::<Vec::<Complex::<T>>>("input");
            let _guard = self.proc_state.lock().unwrap();
            fft_result = self.fft_planner.fft_complex(input_complex.unwrap().as_ref());
        }
        self.get_output("output").unwrap().send(fft_result);
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