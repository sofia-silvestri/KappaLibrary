use std::ffi::{c_char, CStr};

use serde::{Deserialize, Serialize};


pub fn c_char_to_string(c_ptr: *const c_char) -> Result<String, String> {
    if c_ptr.is_null() {
        return Err("Null pointer.".to_string());
    }

    let c_str = unsafe {
        CStr::from_ptr(c_ptr)
    };

    match c_str.to_str() {
        Ok(s) => {
            Ok(s.to_owned()) 
        }
        Err(e) => {
            Err(format!("UTF-8 decoding error: {}", e))
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DependencyFFI {
    pub dep_name: *const c_char,
    pub version: Version,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub dep_name: String,
    pub version: Version,
}
impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}.{}.{}", self.dep_name, self.version.major, self.version.minor, self.version.build)
    }
}
impl From<DependencyFFI> for Dependency {
    fn from(ffi: DependencyFFI) -> Self {
        let name = c_char_to_string(ffi.dep_name).unwrap_or_default();
        Dependency {
            dep_name: name,
            version: ffi.version,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ModuleStructFFI {
    pub name:*const c_char,
    pub description:*const c_char,
    pub authors:*const c_char,
    pub release_date:*const c_char,
    pub version: Version,
    pub dependencies: *const *const DependencyFFI,
    pub dependency_number: usize,
    pub provides: *const *const c_char,
    pub provides_lengths: usize,
}

unsafe impl Sync for ModuleStructFFI {}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModuleStruct {
    pub name: String,
    pub description: String,
    pub authors:String,
    pub release_date:String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
    pub provides: Vec<String>,
}

impl From<ModuleStructFFI> for ModuleStruct {
    fn from(ffi_struct: ModuleStructFFI) -> Self {
        let name = c_char_to_string(ffi_struct.name).unwrap_or_default();
        let description = c_char_to_string(ffi_struct.description).unwrap_or_default();
        let authors = c_char_to_string(ffi_struct.authors).unwrap_or_default();
        let release_date = c_char_to_string(ffi_struct.release_date).unwrap_or_default();

        let mut dependencies = Vec::new();
        for i in 0..ffi_struct.dependency_number {
            let dep_ptr = unsafe { *ffi_struct.dependencies.add(i) };
            let dep_ffi = unsafe { *dep_ptr};
            let dependency: Dependency = dep_ffi.into();
            dependencies.push(dependency);
        }

        let mut provides = Vec::new();
        for i in 0..ffi_struct.provides_lengths {
            let prov_ptr = unsafe { *ffi_struct.provides.add(i) };
            let prov_str = c_char_to_string(prov_ptr).unwrap_or_default();
            provides.push(prov_str);
        }

        ModuleStruct {
            name,
            description,
            authors,
            release_date,
            version: ffi_struct.version,
            dependencies,
            provides,
        }
    }
}

