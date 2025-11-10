use std::os::raw::c_char;
use kappa_library::modules::ModuleStruct;
use kappa_library::modules::Version;

#[unsafe(no_mangle)]
pub static MODULE: ModuleStruct  = ModuleStruct {
    name: "filters_module\0".as_ptr()as *const c_char,
    description: "Library of processing block for signal filtering\0".as_ptr() as *const c_char,
    authors: "Sofia Silvestri\0".as_ptr() as *const c_char,
    release_date: "\0".as_ptr() as *const c_char,
    version: Version{ major: 0,minor: 1,build: 0},
    dependency_number: 0,
    dependencies: &[],
    provides_number: 4,
    provides: &["fir_filter\0".as_ptr() as *const c_char, 
                "iir_filter\0".as_ptr() as *const c_char, 
                "ukf_filter\0".as_ptr() as *const c_char, 
                "ekf_filter\0".as_ptr() as *const c_char],
};

pub mod fir;
pub mod iir;
pub mod ukf;
pub mod ekf;
pub mod cfar_filter;

/*pub mod moving_average;
pub mod median;
pub mod savitzky_golay;
pub mod threshold;
pub mod downsample;
pub mod upsample;
pub mod differentiator;
pub mod integrator;
pub mod notch;
pub mod peak_detector;
pub mod low_pass;
pub mod high_pass;
pub mod band_pass;
pub mod band_stop;
pub mod rms;
pub mod clipping;
pub mod scaler;
pub mod offset;
pub mod normalizer;
pub mod limiter;
pub mod compressor;
pub mod exp_moving_average;
pub mod derivative;
pub mod sliding_window;
pub mod z_score;
pub mod outlier_removal;
pub mod exponential_smoothing;*/
