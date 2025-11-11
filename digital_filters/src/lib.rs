use processor_engine::stream_processor::{StreamBlockDyn, StreamingError};
use data_model::modules::ModuleStruct;
use data_model::modules::Version;

use crate::ekf::EkfProcess;
use crate::fir::FirFilter;
use crate::iir::IirFilter;

#[unsafe(no_mangle)]
pub static MODULE: ModuleStruct  = ModuleStruct {
    name: "filters_module",
    description: "Library of processing block for signal filtering",
    authors: "Sofia Silvestri",
    release_date: "",
    version: Version{ major: 0,minor: 1,build: 0},
    dependency_number: 0,
    dependencies: &[],
    provides_number: 4,
    provides: &["fir_filter",
                "iir_filter",
                "ukf_filter",
                "ekf_filter"],
};

pub mod fir;
pub mod iir;
pub mod ukf;
pub mod ekf;


pub fn get_processor_modules(proc_name: &'static str) -> Result<Box<dyn StreamBlockDyn>, StreamingError> {
    let result: Box<dyn StreamBlockDyn>;
    match proc_name {
        "fir_filter" => {
            result = Box::new(FirFilter::new(Vec::<f64>::new()));
        }
        "iir_filter" => {
            result = Box::new(IirFilter::new(Vec::<f64>::new(),
            Vec::<f64>::new()))}
        "ekf_filter" => {
            result = Box::new(EkfProcess::new(Vec::<f64>::new(), 0.0));
        }
        _ => {
            return Err(StreamingError::InvalidProcessorBlock);
        }
    }
    Ok(result)
}
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
