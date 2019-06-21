//! Thread management

use std::sync::Mutex;

use num_cpus;
use threadpool::ThreadPool;

/// How much to multiply the number of CPUs by to get the default number of
/// threads in a thread pool
static NUM_CPUS_MULTIPLIER: f32 = 2.0;

/// Handle threads within a context
pub struct ThreadManager {
    thread_pool: Mutex<ThreadPool>,
}

impl ThreadManager {
    /// Initialize with a default thread pool size
    pub fn with_default_size(name: String) -> Self {
        let size = (num_cpus::get() as f32 * NUM_CPUS_MULTIPLIER).ceil();
        Self::from_size(name, size as usize)
    }

    /// Initialize with a given thread pool size
    pub fn from_size(name: String, size: usize) -> Self {
        ThreadManager {
            thread_pool: Mutex::new(ThreadPool::with_name(name, size)),
        }
    }

    /// Spawn a new thread (non-blocking)
    pub fn spawn(&self, callback: impl FnOnce() -> () + Send + 'static) {
        self.thread_pool.lock().unwrap().execute(callback)
    }
}
