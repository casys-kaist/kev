//! SMP-supported spinlock.
//!
//! Implementing spinlock for multiprocessor.
//!
//! The implementing unicore spinlock uniprocessor is simple; it just requires preventing thread preemption while holding a lock.
//! By disabling preemption of the lock-holding thread,  other threads cannot access
//! shared resource as they can't be scheduled.
//!
//! However, when it comes to multiprocessor, disabling preemption is not sufficient;
//! as multiple threads run concurrently in different cores, they can access shared resource at the same time
//! even when a core disable preemption.
//! Therefore, to acquire a lock on multi-processor, a processor 1) polls a variable that
//! represents a value is locked or not  2) set the variable when a thread holds the `lock`,
//! and 3) unset the variable when the thread `unlock`.
//!
//! The step 1 and 2 must be executed ATOMICALLY. Therefore, it is required to use
//! atomic read-modify-write instructions supported by CPU.
//! Rust supports abstraction for those instructions through [`core::sync::atomic`] module.
//! You can implements atomic read-modify-write multiple ways through `core::sync::atomic::Atomic*`'s methods.
//! However, we recommend to use `fetch_or` method for simplicity.
//!
//! For the arguments that takes [`Ordering`], just use [`Ordering::SeqCst`].
//! There exists more optimized [`Ordering`] but it is beyond the project's scope.
//! For those who want to know the details, see the <https://en.wikipedia.org/wiki/Memory_ordering>.
//!
//! ## Getting started
//! When you runs following command lines in the spin_lock directory, you can see the test failed message.
//! ```/bin/bash
//! $ cargo test --features=smp
//!
//! ...
//! thread '<unnamed>' panicked at 'not yet implemented', src/smplock.rs:204:9
//! thread '<unnamed>' panicked at 'not yet implemented', src/smplock.rs:204:9
//! ...
//!
//! ```
//!
//! If you go to the 204 line of src/smplock.rs, you can see the `todo!()` macro. You can start
//! your implementation from there.
//!
//! There are several `todo!()` macros, which will guide you about the implemenation order to run
//! the test.
//!
//! After implementing the multiprocessor spinlock, you can boot keos.
//! Continue [`Project 1`].
//!
//! [`core::sync::atomic`]: https://doc.rust-lang.org/core/sync/atomic/index.html
//! [`Ordering`]: https://doc.rust-lang.org/core/sync/atomic/enum.Ordering.html
//! [`Ordering::SeqCst`]: https://doc.rust-lang.org/core/sync/atomic/enum.Ordering.html#variant.SeqCst
//! [`Project 1`]: ../../project1
//!

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// An enumeration of possible errors associated while trying to acquire a lock,
/// from the [`try_lock`] method on a [`SpinLock`].
///
/// [`try_lock`]: SpinLock::try_lock
/// [`SpinLock`]: SpinLock
pub enum TryLockError {
    /// The lock could not be acquired at this time because the operation would
    /// otherwise block.
    WouldBlock,
}

/// A mutual exclusion primitive useful for protecting shared data
///
/// This spinlock will block threads waiting for the lock to become available. The
/// spinlock can be created via a [`new`] constructor. Each spinlock has a type parameter
/// which represents the data that it is protecting. The data can only be accessed
/// through the RAII guards returned from [`lock`] and [`try_lock`], which
/// guarantees that the data is only ever accessed when the spinlock is locked.
///
/// [`new`]: Self::new
/// [`lock`]: Self::lock
/// [`try_lock`]: Self::try_lock
/// [`unwrap()`]: Result::unwrap
///
/// # Examples
///
/// ```
/// use alloc::sync::Arc;
/// use keos::sync::SpinLock;
/// use keos::thread;
///
/// const N: usize = 10;
///
/// // Spawn a few threads to increment a shared variable (non-atomically), and
/// // let the main thread know once all increments are done.
/// //
/// // Here we're using an Arc to share memory among threads, and the data inside
/// // the Arc is protected with a spinlock.
/// let data = Arc::new(SpinLock::new(0));
///
/// for _ in 0..N {
///     let data = Arc::clone(&data);
///     thread::ThreadBuilder::new("work").spawn(move || {
///         // The shared state can only be accessed once the lock is held.
///         // Our non-atomic increment is safe because we're the only thread
///         // which can access the shared state when the lock is held.
///         //
///         // We unwrap() the return value to assert that we are not expecting
///         // threads to ever fail while holding the lock.
///         let mut data = data.lock().unwrap();
///         *data += 1;
///         // the lock is unlocked here when `data` goes out of scope.
///     });
/// }
///
/// ```
///
/// It is sometimes necessary to manually drop the spinlock guard to unlock it
/// sooner than the end of the enclosing scope.
///
/// ```
/// use alloc::sync::{Arc, SpinLock};
/// use keos::thread;
///
/// const N: usize = 3;
///
/// let data_spinlock = Arc::new(SpinLock::new(vec![1, 2, 3, 4]));
/// let res_spinlock = Arc::new(SpinLock::new(0));
///
/// let mut threads = Vec::with_capacity(N);
/// (0..N).for_each(|_| {
///     let data_spinlock_clone = Arc::clone(&data_spinlock);
///     let res_spinlock_clone = Arc::clone(&res_spinlock);
///
///     threads.push(thread::ThreadBuilder::new("work").spawn(move || {
///         let mut data = data_spinlock_clone.lock().unwrap();
///         // This is the result of some important and long-ish work.
///         let result = data.iter().fold(0, |acc, x| acc + x * 2);
///         data.push(result);
///         drop(data);
///         *res_spinlock_clone.lock().unwrap() += result;
///     }));
/// });
///
/// let mut data = data_spinlock.lock().unwrap();
/// // This is the result of some important and long-ish work.
/// let result = data.iter().fold(0, |acc, x| acc + x * 2);
/// data.push(result);
/// // We drop the `data` explicitly because it's not necessary anymore and the
/// // thread still has work to do. This allow other threads to start working on
/// // the data immediately, without waiting for the rest of the unrelated work
/// // to be done here.
/// //
/// // It's even more important here than in the threads because we `.join` the
/// // threads after that. If we had not dropped the spinlock guard, a thread could
/// // be waiting forever for it, causing a deadlock.
/// drop(data);
/// // Here the spinlock guard is not assigned to a variable and so, even if the
/// // scope does not end after this line, the spinlock is still released: there is
/// // no deadlock.
/// *res_spinlock.lock().unwrap() += result;
///
/// threads.into_iter().for_each(|thread| {
///     thread
///         .join()
///         .expect("The thread creating or execution failed !")
/// });
///
/// assert_eq!(*res_spinlock.lock().unwrap(), 800);
/// ```
pub struct SpinLock<T: ?Sized> {
    // Define any member you need.
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new spinlock in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::SpinLock;
    ///
    /// let spinlock = SpinLock::new(0);
    /// ```
    #[inline]
    pub const fn new(t: T) -> SpinLock<T> {
        SpinLock {
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    /// Acquires a spinlock, blocking the current thread until it is able to do so.
    ///
    /// This function will block the local thread until it is available to acquire
    /// the spinlock. Upon returning, the thread is the only thread with the lock
    /// held. An RAII guard is returned to allow scoped unlock of the lock. When
    /// the guard goes out of scope, the spinlock will be unlocked.
    ///
    /// The exact behavior on locking a spinlock in the thread which already holds
    /// the lock is left unspecified. However, this function will not return on
    /// the second call (it might panic or deadlock, for example).
    ///
    /// # Examples
    ///
    /// ```
    /// use alloc::sync::Arc;
    /// use keos::sync::SpinLock;
    /// use keos::thread;
    ///
    /// let spinlock = Arc::new(SpinLock::new(0));
    /// let c_spinlock = Arc::clone(&spinlock);
    ///
    /// thread::spawn(move || {
    ///     *c_spinlock.lock().unwrap() = 10;
    /// }).join().expect("thread::spawn failed");
    /// assert_eq!(*spinlock.lock().unwrap(), 10);
    /// ```
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        todo!()
    }

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then [`Err`] is returned.
    /// Otherwise, an RAII guard is returned. The lock will be unlocked when the
    /// guard is dropped.
    ///
    /// This function does not block.
    ///
    /// # Errors
    ///
    /// If the spinlock could not be acquired because it is already locked, then
    /// this call will return the [`WouldBlock`] error.
    ///
    /// [`WouldBlock`]: TryLockError::WouldBlock
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::SpinLock;
    /// use alloc::sync::Arc;
    /// use keos::thread;
    ///
    /// let spinlock = Arc::new(SpinLock::new(0));
    /// let c_spinlock = Arc::clone(&spinlock);
    ///
    /// thread::spawn(move || {
    ///     let mut lock = c_spinlock.try_lock();
    ///     if let Ok(ref mut spinlock) = lock {
    ///         **spinlock = 10;
    ///     } else {
    ///         println!("try_lock failed");
    ///     }
    /// }).join().expect("thread::spawn failed");
    /// assert_eq!(*spinlock.lock().unwrap(), 10);
    /// ```
    pub fn try_lock(&self) -> Result<SpinLockGuard<'_, T>, TryLockError> {
        todo!()
    }

    /// Consumes this spinlock, returning the underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::SpinLock;
    ///
    /// let spinlock = SpinLock::new(0);
    /// assert_eq!(spinlock.into_inner().unwrap(), 0);
    /// ```
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.data.into_inner()
    }
}

impl<T: ?Sized + Default> Default for SpinLock<T> {
    /// Creates a `SpinLock<T>`, with the `Default` value for T.
    fn default() -> SpinLock<T> {
        SpinLock::new(Default::default())
    }
}

impl<T: ?Sized> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

/// An RAII implementation of a "scoped lock" of a spinlock. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// [`Deref`] and [`DerefMut`] implementations.
///
/// This structure is created by the [`lock`] and [`try_lock`] methods on
/// [`SpinLock`].
///
/// [`lock`]: SpinLock::lock
/// [`try_lock`]: SpinLock::try_lock
pub struct SpinLockGuard<'a, T: ?Sized + 'a> {
    lock: &'a SpinLock<T>,
}

impl<T: ?Sized> !Send for SpinLockGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinLockGuard<'_, T> {}

impl<T: ?Sized> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::SpinLock;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread::scope;

    #[test]
    pub fn smoke() {
        const LENGTH: usize = 1024;
        let lock = SpinLock::new(vec![]);
        let start = Arc::new(AtomicBool::new(true));

        scope(|s| {
            for i in 0..LENGTH {
                let d = &lock;
                let start = start.clone();
                s.spawn(move || {
                    while start.load(Ordering::SeqCst) {}
                    let mut d = d.lock();
                    d.push(i);
                });
            }
            start.store(false, Ordering::SeqCst);
        });

        lock.lock().sort();
        assert_eq!(lock.into_inner(), (0..LENGTH).collect::<Vec<_>>());
    }
}
