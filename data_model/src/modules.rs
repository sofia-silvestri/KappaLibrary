use libloading::Library;

use processor_engine::stream_processor::StreamProcessor;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub build: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Dependency {
    pub dep_name: &'static str,
    pub version: Version,
}

pub trait ModulesTrait {
    fn get_processor_modules(&self, name: &str) -> Box<dyn StreamProcessor>;
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ModuleStruct {
    pub name: &'static str,
    pub description: &'static str,
    pub authors: &'static str,
    pub release_date: &'static str,
    pub version: Version,
    pub dependency_number: u32,
    pub dependencies: &'static [&'static Dependency],
    pub provides_number: u32,
    pub provides: &'static [&'static str],
}

#[repr(C)]
pub struct ModuleHandle {
    pub module: ModuleStruct,
    pub _lib: Library,
}

unsafe impl Sync for ModuleStruct {}
