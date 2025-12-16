use std::{ffi::*, mem};
use crate::{modules::{ModuleStruct,ModuleStructFFI}, streaming_data::StreamErrCode};
use libloading::{Library, Symbol};

#[repr(C)]
pub struct FfiStrSlice {
    /// Puntatore al primo puntatore c_char. Tipo C: const char**
    pub data: *const *const c_char,
    /// Lunghezza dell'array.
    pub len: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct ModuleHandle<'a> {
    pub module: ModuleStruct,
    pub lib: &'a Library,
    pub get_processor_modules: Symbol<'static, unsafe extern "C" fn(*const u8, usize, *const u8, usize) -> TraitObjectRepr>,
}

impl ModuleHandle<'static> {
    pub fn new(library_path: String) -> Result<Self, StreamErrCode> {
        let library: &'static Library;
        match unsafe { Library::new(library_path)} {
            Ok(lib) => {
                let box_library = Box::leak(Box::new(lib));
                library = box_library;
            }
            Err(_) => {
                eprintln!("Unable to find");
                return Err(StreamErrCode::FileNotFound);
            }
        }
        let module_info: Symbol<*mut ModuleStructFFI>;
        match unsafe { library.get(b"MODULE\0") } {
            Ok(module) => {module_info = module;}
            Err(_) => {
                eprintln!("Unable to find");
                return Err(StreamErrCode::FileNotFound);
            }
        }
        let module = unsafe{**module_info}.into();
        let funct_ptr: Symbol<unsafe extern "C" fn(*const u8, usize, *const u8, usize) -> TraitObjectRepr>;
        match unsafe { library.get(b"get_processor_modules")} {
            Ok(func) => {funct_ptr = func;}
            Err(_) => {
                eprintln!("Unable to find");
                return Err(StreamErrCode::FileNotFound);
            }
        }
        let handle = Self {
            lib: library,
            module: module,
            get_processor_modules: funct_ptr,
        };
        Ok(handle)
    }
}

// Representazione C-compatible of trait object
#[repr(C)]
pub struct TraitObjectRepr {
    pub data: *mut c_void,
    pub vtable: *mut c_void,
}
pub fn get_error_return(code: i32) -> TraitObjectRepr {
    TraitObjectRepr {
        data: code as *mut c_void,
        vtable: std::ptr::null_mut(),
    }
}
// Funzione FFI per creare e restituire l'oggetto
/*pub fn export_stream_processor(proc: Box<dyn StreamProcessor>) -> TraitObjectRepr {
    
    let ptr_fat: *mut dyn StreamProcessor = Box::into_raw(proc);
    
    unsafe {
        mem::transmute(ptr_fat)
    }
}

// Funzione FFI per usare l'oggetto
pub fn import_stream_processor(repr: TraitObjectRepr) -> Box<dyn StreamProcessor> {
    // SAFETY: Dobbiamo essere in un blocco unsafe per riconvertire la repr 
    // nel puntatore a trait object originale tramite transmute.
    unsafe {
        let trait_heap_pointer: *mut dyn StreamProcessor = mem::transmute(repr);
        Box::from_raw(trait_heap_pointer) 
        
    }
}

pub fn free_object(repr: TraitObjectRepr) {
    // SAFETY: il chiamante deve garantire che la repr sia valida e non ancora liberata.
    unsafe {
        // Riconvertire la repr in un puntatore a trait object originale
        let trait_heap_pointer: *mut dyn StreamProcessor = mem::transmute(repr);
        
        // Box::from_raw riprende la proprietà del Box originale e lo dealloca
        let _boxed_trait: Box<dyn StreamProcessor> = Box::from_raw(trait_heap_pointer);
    }
}*/