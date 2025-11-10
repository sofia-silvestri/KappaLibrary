use chrono::Utc;
use kappa_library::stream_processor::{StreamProcessor, StreamBlock};
use filter::ekf::{EkfProcess, TimeTaggedSample};
use rand::rand_core::le;

fn main() {
    let file_path = "filter/test/ekf_noisy_spiral.bin";
    use std::path::Path;
    use std::fs;
    use std::slice;
    let init_state = vec![0.0, 0.0, 0.0, 0.0];
    let current_dir = std::env::current_dir().unwrap();
    let full_path = current_dir.join(file_path);
    let file_path = full_path.to_str().unwrap();
    if !Path::new(file_path).exists() {
        let time_factor: f64 = 0.01;
        let number_of_samples = 10000;
        let mut data: Vec<f64> = Vec::new();
        let mut value: f64 = 0.0;
        for n in 0..number_of_samples {
            value = (n as f64 * time_factor).cos() + 0.5*rand::random::<f64>();
            // Simple motion model: constant velocity
            data.push(value);
        }

        let total_bytes = data.len() * 8;
        let byte_slice: &[u8] = unsafe {
            let ptr = data.as_ptr();
            let u8_ptr = ptr as *const u8;
            std::slice::from_raw_parts(u8_ptr, total_bytes)
        };
        let write_result = fs::write(file_path, byte_slice);
        match write_result {
            Ok(_) => println!("Test input data written to {}", file_path),
            Err(e) => panic!("Failed to write test input data: {}", e),
        }
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
    let mut process_noise = vec![vec![0.0; 4]; 4];
    for i in 0..4 {
        process_noise[i][i] = 50.0;
    }
    let measurement_noise = 5.0;
    ekf_process.set_parameter_value("process_noise", process_noise).unwrap();
    ekf_process.set_parameter_value("measurement_noise", measurement_noise).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    ekf_process.connect::<TimeTaggedSample<f64>>("output", tx);
    let mut result_vec : Vec<f64> = Vec::new();
    for &value in data_input.iter() {
        println!("Feeding input value: {:?}", value);
        let input_sample = Some(TimeTaggedSample {
            time: Utc::now(),
            value,
        });
        ekf_process.get_input_channel("input").unwrap().send(input_sample);
        ekf_process.process().unwrap();
        let output = rx.recv().unwrap();
        assert!(output.value.is_finite());
        result_vec.push(output.value);
    }
    let total_bytes = result_vec.len() * 8;
    let f64_count = result_vec.len();
    let byte_slice: &[u8] = unsafe {
        let ptr = result_vec.as_ptr();
        let u8_ptr = ptr as *const u8;
        std::slice::from_raw_parts(u8_ptr, total_bytes)
    };
    fs::write("filter/test/ekf_noisy_spiral_output.bin", byte_slice);
}
