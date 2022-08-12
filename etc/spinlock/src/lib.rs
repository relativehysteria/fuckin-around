//! A spinlock implementation. Similar to std's Mutex.

#![no_std]

use core::sync::atomic::{ AtomicU32, Ordering };
use core::ops::{ Deref, DerefMut };
use core::cell::UnsafeCell;
use core::hint::spin_loop;

/// A spinlock-guarded inner-mutable variable
#[repr(C)]
pub struct SpinLock<T: ?Sized> {
    /// Ticket counter. A ticket is grabbed and when `release` is set to this
    /// ticket, you get your variable
    ticket: AtomicU32,

    /// Current `ticket` value which can be released
    release: AtomicU32,

    /// Value guarded by this lock
    val: UnsafeCell<T>
}

// Mark the SpinLock Send and Sync
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Move a `val` into a `SpinLock`
    pub const fn new(val: T) -> Self {
        Self {
            val:     UnsafeCell::new(val),
            ticket:  AtomicU32::new(0),
            release: AtomicU32::new(0),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    /// Acquire exclusive acces to the variable
    pub fn lock(&self) -> SpinLockGuard<T> {
        let ticket = self.ticket.fetch_add(1, Ordering::SeqCst);

        while self.release.load(Ordering::SeqCst) != ticket {
            spin_loop();
        }

        SpinLockGuard {
            lock: &self,
        }
    }

    /// Return a raw pointer to the internal locked value, bypassing the lock.
    pub unsafe fn shatter(&self) -> *mut T {
        self.val.get()
    }
}

/// A guard structure which implements `Drop` so the locks can be released
/// based on scope
pub struct SpinLockGuard<'a, T: ?Sized> {
    /// Reference to the value that we currently have exclusive acces to
    lock: &'a SpinLock<T>,
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.release.fetch_add(1, Ordering::SeqCst);
    }
}

impl<'a, T: ?Sized> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self.lock.val.get()
        }
    }
}

impl<'a, T: ?Sized> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.lock.val.get()
        }
    }
}
