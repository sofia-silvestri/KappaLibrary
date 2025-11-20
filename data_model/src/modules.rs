use std::ffi::c_char;

use libloading::Library;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Dependency {
    pub dep_name: *const c_char,
    pub version: Version,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ModuleStruct {
    pub name:*const c_char,
    pub description:*const c_char,
    pub authors:*const c_char,
    pub release_date:*const c_char,
    pub version: Version,
    pub dependencies: *const *const Dependency,
    pub dependency_number: usize,
    pub provides: *const *const c_char,
    pub provides_lengths: usize,
}

unsafe impl Sync for ModuleStruct {}

