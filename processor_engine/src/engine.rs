use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::fmt;
use libc::{pthread_self, pthread_t, pthread_getcpuclockid, clockid_t};

struct Task {
    pub name: &'static str,
    thread_id: pthread_t,
    cpu_clock_id: clockid_t,
}

impl Task {
    pub fn new(name: &'static str, thread_id: pthread_t) -> Self {
        
        let mut cpu_clock_id: clockid_t = 0;
        unsafe {
            pthread_getcpuclockid(thread_id, &mut cpu_clock_id);
        }
        Task {
            name,
            thread_id,
            cpu_clock_id,
        }
    }

}

pub struct TaskManager {
    tasks: HashMap<&'static str, Task>,
}

impl TaskManager {
    fn new() -> Self {
        TaskManager {
            tasks: HashMap::new(),
        }
    }
    pub fn get() -> &'static Mutex<TaskManager> {
        TASK_MANAGER.get_or_init(|| Mutex::new(TaskManager::new()))
    }
    pub fn create_task<F, T, S: Copy>(&mut self, name: S, f: F) -> std::io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T + Send + 'static, 
        T: Send + 'static, 
        S: Into<String> + fmt::Display, 
    {
        let builder = thread::Builder::new().name(name.into());  
        let thread_id = unsafe { pthread_self() };
        let name: &'static str = Box::leak(Box::new(name.to_string()));
        let task = Task::new(name, thread_id);
        self.tasks.insert(name, task);
        builder.spawn(f)
    }
}

static TASK_MANAGER: OnceLock<Mutex<TaskManager>> = OnceLock::new();
