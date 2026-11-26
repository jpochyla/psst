//! Bounded network concurrency and per-request coalescing, inspired by Spotifast.
use parking_lot::{Condvar, Mutex};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use ureq::{http::Response, Body};

struct State {
    active: usize,
    next: Instant,
}
struct Gate {
    state: Mutex<State>,
    changed: Condvar,
    spacing: Duration,
}

pub(super) struct Dispatcher(Arc<Gate>);
impl Default for Dispatcher {
    fn default() -> Self {
        Self(Arc::new(Gate {
            state: Mutex::new(State {
                active: 0,
                next: Instant::now(),
            }),
            changed: Condvar::new(),
            spacing: Duration::from_millis(300),
        }))
    }
}
impl Dispatcher {
    pub fn acquire(&self) -> Permit {
        let mut state = self.0.state.lock();
        loop {
            if state.active >= 2 {
                self.0.changed.wait(&mut state);
                continue;
            }
            if let Some(wait) = state.next.checked_duration_since(Instant::now()) {
                self.0.changed.wait_for(&mut state, wait);
                continue;
            }
            state.active += 1;
            state.next = Instant::now() + self.0.spacing;
            return Permit(self.0.clone());
        }
    }
}
pub(super) struct Permit(Arc<Gate>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.state.lock().active -= 1;
        self.0.changed.notify_all();
    }
}

/// The permit lives until the response body is read or dropped, including errors.
pub(super) struct ApiResponse {
    response: Response<Body>,
    _permit: Permit,
}
impl ApiResponse {
    pub fn new(response: Response<Body>, permit: Permit) -> Self {
        Self {
            response,
            _permit: permit,
        }
    }
}
impl Deref for ApiResponse {
    type Target = Response<Body>;
    fn deref(&self) -> &Self::Target {
        &self.response
    }
}
impl DerefMut for ApiResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

#[derive(Default)]
pub(super) struct Loads(Mutex<HashMap<String, Weak<Mutex<()>>>>);
impl Loads {
    pub fn for_key(&self, key: String) -> Arc<Mutex<()>> {
        let mut loads = self.0.lock();
        loads.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = loads.get(&key).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(Mutex::new(()));
        loads.insert(key, Arc::downgrade(&lock));
        lock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slow_request_does_not_block_a_second_but_a_third_waits_for_capacity() {
        let dispatcher = Arc::new(Dispatcher::default());
        let first = dispatcher.acquire();
        let second = dispatcher.acquire();
        let (tx, rx) = std::sync::mpsc::channel();
        let other = dispatcher.clone();
        let thread = std::thread::spawn(move || {
            let permit = other.acquire();
            tx.send(()).unwrap();
            drop(permit);
        });
        assert!(rx.recv_timeout(Duration::from_millis(30)).is_err());
        drop(first);
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(second);
        thread.join().unwrap();
        assert_eq!(dispatcher.0.state.lock().active, 0);
    }
    #[test]
    fn equivalent_reads_share_a_lock_but_other_cached_pages_remain_accessible() {
        let loads = Loads::default();
        let first = loads.for_key("account-a:playlist".into());
        let same = loads.for_key("account-a:playlist".into());
        let other = loads.for_key("account-a:album".into());
        let account = loads.for_key("account-b:playlist".into());
        let held = first.lock();
        assert!(same.try_lock().is_none());
        assert!(other.try_lock().is_some());
        assert!(account.try_lock().is_some());
        drop(held);
        drop(first);
        drop(same);
        drop(other);
        drop(account);
        let _next = loads.for_key("next".into());
        assert_eq!(loads.0.lock().len(), 1);
    }
}
