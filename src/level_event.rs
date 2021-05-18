use std::sync::{Arc, Condvar, Mutex};
use std::time;

#[derive(Clone)]
pub struct LevelEvent {
    flag: Arc<(Mutex<bool>, Condvar)>,
}

impl LevelEvent {
    pub fn new() -> LevelEvent {
        LevelEvent {
            flag: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn test(&self) -> bool {
        *self.flag.0.lock().unwrap()
    }

    pub fn wait(&self) {
        let &(ref lock, ref condvar) = &*self.flag;
        let _lock = condvar
            .wait_while(lock.lock().unwrap(), |flag| !*flag)
            .unwrap();
    }

    pub fn wait_timeout(&self, timeout: time::Duration) -> bool {
        let &(ref lock, ref condvar) = &*self.flag;
        let lock = condvar
            .wait_timeout_while(lock.lock().unwrap(), timeout, |flag| !*flag)
            .unwrap();
        *lock.0
    }

    pub fn activate(&mut self) {
        let &(ref lock, ref condvar) = &*self.flag;
        *lock.lock().unwrap() = true;
        condvar.notify_all();
    }
}
