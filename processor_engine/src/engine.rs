use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::fmt;
use chrono::{DateTime, Utc};
use data_model::streaming_error::StreamingError;
use libc::{clock_gettime, clockid_t, pthread_getcpuclockid, pthread_self, pthread_t, timespec};

#[repr(C)]
#[derive(Clone, Copy)]
struct Task {
    pub name: &'static str,
    thread_id: pthread_t,
    cpu_clock_id: clockid_t,
    last_cpu_time: f64,
    last_update: DateTime<Utc>
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
            last_cpu_time: 0.0,
            last_update: Utc::now(),
        }
    }
    pub fn update(&mut self) -> Result<f64, StreamingError> {
        let mut occupacy: f64 = 0.0;
        unsafe {
            let mut ts: timespec = timespec { tv_sec: 0, tv_nsec: 0 };
            let ts_ptr: *mut timespec = &mut ts as *mut timespec;
            if clock_gettime(self.cpu_clock_id, ts_ptr) != 0 {
                return Err(StreamingError::TaskError);
            }
            if self.last_cpu_time == 0.0 {
                self.last_cpu_time = utils::time::timespec_to_f64(&ts);
                self.last_update = Utc::now();
            } else {
                let current_cpu_time = utils::time::timespec_to_f64(&ts);
                let current_time = Utc::now();
                let cpu_time_diff = current_cpu_time - self.last_cpu_time;
                let wall_time_diff = (current_time - self.last_update).num_nanoseconds().unwrap() as f64 * 1e-9;
                if wall_time_diff > 0.0 {
                    occupacy = cpu_time_diff / wall_time_diff;
                }
                self.last_cpu_time = current_cpu_time;
                self.last_update = current_time;
            }
        }
        Ok(occupacy)
    }
}

pub struct TaskManager {
    tasks: HashMap<&'static str, Task>,
    thread_occupacy: HashMap<&'static str, f64>
    interval_secs: f64,
}

impl TaskManager {
    fn new() -> Self {
        TaskManager {
            tasks: HashMap::new(),
            thread_occupacy: HashMap::new(),
            interval_secs: 0.1,
        }
    }
    pub fn get() -> &'static Mutex<TaskManager> {
        TASK_MANAGER.get_or_init(|| Mutex::new(TaskManager::new()))
    }
    pub fn set_time_update(&mut self, interval_secs: f64) {
        self.interval_secs = interval_secs;
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
        self.thread_occupacy.insert(name, 0.0);
        builder.spawn(f)
    }
    pub fn start_task_monitoring(&self, ) {
        thread::spawn(move || {
            loop {
                let task_manager = TaskManager::get();
                thread::sleep(std::time::Duration::from_secs_f64(task_manager.lock().unwrap().interval_secs));
                for (name, task) in task_manager.lock().unwrap().tasks.iter_mut() {
                    match task.update() {
                        Ok(occupacy) => {
                            task_manager.lock().unwrap().thread_occupacy.insert(&name, occupacy);
                        }
                        Err(e) => {
                            eprintln!("Error updating task '{}': {}", name, e);
                        }
                    }
                }
            }
        });
    }
}

static TASK_MANAGER: OnceLock<Mutex<TaskManager>> = OnceLock::new();
