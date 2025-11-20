pub mod fourier;

use std::ffi::{c_char, c_void};

use data_model::modules::{Version,ModuleStruct};
use processor_engine::stream_processor::{StreamBlockDyn, StreamProcessor};
use processor_engine::ffi::{TraitObjectRepr, export_stream_processor, get_error_return};

#[unsafe(no_mangle)]
pub static MODULE: ModuleStruct  = ModuleStruct {
    name: b"digital_transform\0".as_ptr() as *const c_char,
    description: b"Library of processing block for signal transformation in frequency domain\0".as_ptr() as *const c_char,
    authors: b"Sofia Silvestri\0".as_ptr() as *const c_char,
    release_date: b"\0".as_ptr() as *const c_char,
    version: Version{ major: 0,minor: 1,build: 0},
    dependencies: std::ptr::null(),
    dependency_number: 0,
    provides: [
        b"fft_f64\0".as_ptr() as *const c_char,
        b"fft_f32\0".as_ptr() as *const c_char,
    ].as_ptr(),
    provides_lengths: 2,
};


#[unsafe(no_mangle)]
pub extern "C" fn get_processor_modules(proc_block: *const u8, 
    proc_block_len: usize, 
    block_name: *const u8, 
    block_name_len: usize) -> TraitObjectRepr {
    let proc_block_str = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(proc_block, proc_block_len)).unwrap()
    };
    let block_name_str = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(block_name, block_name_len)).unwrap()
    };  
    let proc: Box<dyn StreamProcessor>;
    println!("Requested block: {}", proc_block_str);
    match proc_block_str {
        "fft_f64" => {
            proc = Box::new(fourier::FftProcess::<f64>::new(block_name_str));
            println!("Exporting block: {}", proc_block_str);
            export_stream_processor(proc)
        }
        "fft_f32" => {
            proc = Box::new(fourier::FftProcess::<f32>::new(block_name_str));
            println!("Exporting block: {}", proc_block_str);
            export_stream_processor(proc)
        }
        _ => {
            eprintln!("Processor block {} not found", proc_block_str);
            get_error_return(1)
        }
    }
}
