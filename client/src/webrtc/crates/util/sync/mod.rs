/// A synchronous mutual exclusion primitive useful for protecting shared data
pub(crate) type Mutex<T> = parking_lot::Mutex<T>;

/// A synchronous reader-writer lock
pub(crate) type RwLock<T> = parking_lot::RwLock<T>;