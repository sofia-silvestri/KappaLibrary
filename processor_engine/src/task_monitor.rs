use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock, Arc};
use std::thread::{self, JoinHandle};
use std::fmt;
use chrono::{DateTime, Utc};
use data_model::streaming_data::StreamErrCode;
use libc::{clock_gettime, clockid_t, pthread_getcpuclockid, pthread_self, pthread_t, timespec};
use utils::math::statistics::{mean, std_deviation, percentile};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskStatistics {
    pub timestamp: f64,
    pub mean: f64,
    pub max: f64,
    pub min: f64,
    pub std_dev: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
}

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

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

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
    pub fn update(&mut self) -> Result<(), StreamErrCode> {
        let mut occupacy: f64 = 0.0;
        unsafe {
            let mut ts: timespec = timespec { tv_sec: 0, tv_nsec: 0 };
            let ts_ptr: *mut timespec = &mut ts as *mut timespec;
            if clock_gettime(self.cpu_clock_id, ts_ptr) != 0 {
                return Err(StreamErrCode::TaskError);
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
    pub fn get_stats(&self) -> TaskStatistics {
        let timestamp = Utc::now();
        let mut data: Vec<f64> = self.occupacy.iter().cloned().collect();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = mean::<f64>(data.clone());
        let max = *data.last().unwrap_or(&0.0);
        let min = *data.first().unwrap_or(&0.0);
        let std_dev = std_deviation::<f64>(data.clone(), mean);
        let p50 = percentile::<f64>(&mut data.clone(), 50.0);
        let p90 = percentile::<f64>(&mut data.clone(), 90.0);
        let p99 = percentile::<f64>(&mut data, 99.0);
        TaskStatistics {
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
    thread_statics: HashMap<&'static str, TaskStatistics>,
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
        TASK_MANAGER.get_or_init(|| Arc::new(Mutex::new(TaskManager::new())))
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
    pub fn create_task<F, T, S: Clone>(&mut self, name: S, f: F) -> std::io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T + Send + 'static, 
        T: Send + 'static, 
        S: Into<String> + fmt::Display, 
    {
        let builder = thread::Builder::new().name(name.clone().into());  
        let thread_id = unsafe { pthread_self() };
        let name: &'static str = Box::leak(Box::new(name.to_string().clone()));
        let task = Task::new(name, thread_id);
        self.tasks.insert(name, task); 
        self.thread_statics.insert(name, TaskStatistics {
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
}

pub static TASK_MANAGER: OnceLock<Arc<Mutex<TaskManager>>> = OnceLock::new();

pub fn start_task_monitoring() -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let mut task_manager = TaskManager::get().lock().unwrap();
            let mut update_statistics = false;
            let b = task_manager.interval_update;
            thread::sleep(std::time::Duration::from_secs_f64(b));
            task_manager.count_updates += 1;
            if (task_manager.count_updates % task_manager.interval_statistics) == 0 {
                update_statistics = true;
                task_manager.count_updates = 0;
            }
            let mut stats_temp: HashMap<&'static str, TaskStatistics> = HashMap::new();
            for (name, task) in task_manager.tasks.iter_mut() {
                match task.update() {
                    Ok(_) => {
                        if !update_statistics {
                            continue;
                        }
                        let stats = task.get_stats();
                        stats_temp.insert(&name, stats);
                    }
                    Err(e) => {
                        eprintln!("Error updating task '{}': {}", name, e);
                    }
                }
            }
            for (name, stats) in stats_temp.iter() {
                task_manager.thread_statics.insert(&name, *stats);
            }
            if task_manager.send_statistics && update_statistics {
                todo!(); // Send statistics to monitoring system
            }   
        }
    })
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_task_monitor() {
        let handle: JoinHandle<()>;
        {
            let mut task_manager = TaskManager::get().lock().unwrap();
            task_manager.set_time_update(0.01);
            task_manager.set_statistics_interval(1.0);
            handle = task_manager.create_task("test_task", || {
                    for _ in 0..10 {
                        thread::sleep(std::time::Duration::from_millis(250));
                    }
            }).unwrap();
        }
        
        
        start_task_monitoring();
        handle.join().unwrap();
        let task_manager = TaskManager::get().lock().unwrap();
        let stats = task_manager.thread_statics.get("test_task").unwrap();
        println!("Task Statistics - Mean: {}, Max: {}, Min: {}, Std Dev: {},  P50: {}, P90: {}, P99: {}",
            stats.mean, stats.max, stats.min, stats.std_dev, stats.p50, stats.p90, stats.p99);
        assert!(stats.mean >= 0.0);
    }
}