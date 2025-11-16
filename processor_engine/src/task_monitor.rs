use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::fmt;
use chrono::{DateTime, Utc};
use data_model::streaming_data::StreamingError;
use libc::{clock_gettime, clockid_t, pthread_getcpuclockid, pthread_self, pthread_t, timespec};
use utils::math::statistics::{mean, std_deviation, percentile};
use data_model::streaming_data::KappaStatistics;

#[repr(C)]
#[derive(Clone)]
struct Task {
    pub name: &'static str,
    pub occupacy: VecDeque<f64>,
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
            occupacy: VecDeque::with_capacity(100),
            thread_id,
            cpu_clock_id,
            last_cpu_time: 0.0,
            last_update: Utc::now(),
        }
    }
    pub fn update(&mut self) -> Result<(), StreamingError> {
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
                self.occupacy.push_back(occupacy);
                if self.occupacy.len() > 100 {
                    self.occupacy.pop_front();
                }
                self.last_cpu_time = current_cpu_time;
                self.last_update = current_time;
            }
        }
        Ok(())
    }
    pub fn get_stats(&self) -> KappaStatistics {
        let timestamp = Utc::now();
        let mut data: Vec<f64> = self.occupacy.iter().cloned().collect();
        let mean = mean::<f64>(data.clone());
        let max = *data.last().unwrap_or(&0.0);
        let min = *data.first().unwrap_or(&0.0);
        let std_dev = std_deviation::<f64>(data.clone(), mean);
        let p50 = percentile::<f64>(&mut data.clone(), 50.0);
        let p90 = percentile::<f64>(&mut data.clone(), 90.0);
        let p99 = percentile::<f64>(&mut data, 99.0);
        KappaStatistics {
            timestamp: timestamp.timestamp_millis() as f64 * 1e-3,
            mean,
            max,
            min,
            std_dev,
            p50,
            p90,
            p99,
        }
    }
}

pub struct TaskManager {
    tasks: HashMap<&'static str, Task>,
    thread_statics: HashMap<&'static str, KappaStatistics>,
    interval_update: f64,
    interval_statistics: usize,
    send_statistics: bool,
    count_updates: usize,
}

impl TaskManager {
    fn new() -> Self {
        TaskManager {
            tasks: HashMap::new(),
            thread_statics: HashMap::new(),
            interval_update: 0.1,
            interval_statistics: 10,
            send_statistics: false,
            count_updates: 0,
        }
    }
    pub fn get() -> &'static Mutex<TaskManager> {
        TASK_MANAGER.get_or_init(|| Mutex::new(TaskManager::new()))
    }
    pub fn set_time_update(&mut self, interval_update: f64) {
        self.interval_update = interval_update;
    }
    pub fn enable_statistics_sending(&mut self, enable: bool) {
        self.send_statistics = enable;
    }
    pub fn set_statistics_interval(&mut self, interval_statistics: f64) {
        self.interval_statistics = (interval_statistics/self.interval_update) as usize;
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
        self.thread_statics.insert(name, KappaStatistics {
            timestamp: Utc::now().timestamp_millis() as f64 * 1e-3,
            mean: 0.0,
            max: 0.0,
            min: 0.0,
            std_dev: 0.0,
            p50: 0.0,
            p90: 0.0,
            p99: 0.0,
        });
        builder.spawn(f)
    }
    pub fn start_task_monitoring(&self, ) {
        thread::spawn(move || {
            loop {
                let task_manager = TaskManager::get();
                let mut update_statistics = false;
                thread::sleep(std::time::Duration::from_secs_f64(task_manager.lock().unwrap().interval_update));
                task_manager.lock().unwrap().count_updates += 1;
                if (task_manager.lock().unwrap().count_updates % task_manager.lock().unwrap().interval_statistics) == 0 {
                    update_statistics = true;
                    task_manager.lock().unwrap().count_updates = 0;
                }
                for (name, task) in task_manager.lock().unwrap().tasks.iter_mut() {
                    match task.update() {
                        Ok(_) => {
                            if !update_statistics {
                                continue;
                            }
                            let stats = task.get_stats();
                            task_manager.lock().unwrap().thread_statics.insert(&name, stats);
                        }
                        Err(e) => {
                            eprintln!("Error updating task '{}': {}", name, e);
                        }
                    }
                }
                if task_manager.lock().unwrap().send_statistics && update_statistics {
                    todo!(); // Send statistics to monitoring system
                }   
            }
        });
    }
}

static TASK_MANAGER: OnceLock<Mutex<TaskManager>> = OnceLock::new();