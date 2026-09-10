use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct TaskHandle {
    id: u64,
    cancelled: Arc<AtomicBool>,
}

impl TaskHandle {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Default)]
pub struct TaskSupervisor {
    next_id: Arc<AtomicU64>,
    tasks: Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>,
}

impl TaskSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self) -> TaskHandle {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.tasks
            .lock()
            .expect("task supervisor poisoned")
            .insert(id, Arc::clone(&cancelled));
        TaskHandle { id, cancelled }
    }

    pub fn finish(&self, handle: &TaskHandle) {
        self.tasks
            .lock()
            .expect("task supervisor poisoned")
            .remove(&handle.id);
    }

    pub fn cancel_all(&self) {
        for token in self
            .tasks
            .lock()
            .expect("task supervisor poisoned")
            .values()
        {
            token.store(true, Ordering::Release);
        }
    }

    pub fn active_count(&self) -> usize {
        self.tasks.lock().expect("task supervisor poisoned").len()
    }
}
