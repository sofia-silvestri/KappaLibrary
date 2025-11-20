use std::{ffi::*, mem};
use data_model::modules::ModuleStruct;
use libloading::Library;
use crate::stream_processor::StreamProcessor;

#[repr(C)]
pub struct FfiStrSlice {
    /// Puntatore al primo puntatore c_char. Tipo C: const char**
    pub data: *const *const c_char,
    /// Lunghezza dell'array.
    pub len: usize,
}

#[repr(C)]
pub struct ModuleHandle {
    pub module: ModuleStruct,
    pub _lib: Library,
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
pub fn export_stream_processor(proc: Box<dyn StreamProcessor>) -> TraitObjectRepr {
    
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
}


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