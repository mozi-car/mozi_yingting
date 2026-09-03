use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Task {
    pub data: Arc<Mutex<Vec<u8>>>,
    pub stop: Arc<Mutex<bool>>,
    pub worker: Option<JoinHandle<()>>,
}
pub type Tasks = Arc<Mutex<HashMap<String, Task>>>;

pub fn stop(tasks: &Tasks, id: &str) -> bool {
    let mut map = match tasks.lock() {
        Ok(value) => value,
        Err(_) => return false,
    };
    if let Some(mut task) = map.remove(id) {
        if let Ok(mut value) = task.stop.lock() {
            *value = true;
        }
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        true
    } else {
        false
    }
}

pub fn spawn(tasks: &Tasks, id: String, period: Duration, data: Vec<u8>) {
    let shared = Arc::new(Mutex::new(data));
    let stop = Arc::new(Mutex::new(false));
    let loop_data = shared.clone();
    let loop_stop = stop.clone();
    let worker = thread::spawn(move || loop {
        std::thread::sleep(period);
        if loop_stop.lock().map(|v| *v).unwrap_or(true) {
            break;
        }
        drop(loop_data.lock());
    });
    if let Ok(mut map) = tasks.lock() {
        map.insert(
            id,
            Task {
                data: shared,
                stop,
                worker: Some(worker),
            },
        );
    }
}
