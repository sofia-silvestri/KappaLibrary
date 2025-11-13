use std::collections::HashMap;
use std::thread::{self, JoinHandle};
use std::fmt;

// Definizione della funzione wrapper
pub fn spawn_named<F, T, S>(name: S, f: F) -> std::io::Result<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static, 
    T: Send + 'static, 
    S: Into<String> + fmt::Display, 
{
    let builder = thread::Builder::new().name(name.into());   
    builder.spawn(f)
}

pub struct Engine {
    
}