use std::sync::{LockResult, MutexGuard, RwLockReadGuard, RwLockWriteGuard};

pub struct Mutex<T>(std::sync::Mutex<T>);
pub struct RwLock<T>(std::sync::RwLock<T>);

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self(std::sync::Mutex::new(value))
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        unwrap_poison(self.0.lock())
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self(std::sync::RwLock::new(value))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        unwrap_poison(self.0.read())
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        unwrap_poison(self.0.write())
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

fn unwrap_poison<G>(res: LockResult<G>) -> G {
    match res {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}
