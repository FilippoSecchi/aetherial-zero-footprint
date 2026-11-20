use std::ops::{Deref, DerefMut};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Failed to lock memory pages in RAM (os error: {0})")]
    LockFailed(std::io::Error),
    #[error("Buffer size cannot be zero")]
    EmptyBuffer,
}

/// Volatile protected memory buffer:
/// 1. Locks pages in physical RAM preventing disk swap (mlock / VirtualLock).
/// 2. Deterministically overwrites bytes with zeros on Drop (zeroize).
/// 3. Unlocks pages before returning memory to the allocator.
pub struct LockedBuffer {
    data: Vec<u8>,
    is_locked: bool,
}

impl LockedBuffer {
    pub fn new(size: usize) -> Result<Self, MemoryError> {
        if size == 0 {
            return Err(MemoryError::EmptyBuffer);
        }

        let mut data = vec![0u8; size];
        let is_locked = Self::lock_pages(data.as_mut_ptr(), data.len())?;

        Ok(Self { data, is_locked })
    }

    pub fn from_secret_slice(secret: &mut [u8]) -> Result<Self, MemoryError> {
        let mut buf = Self::new(secret.len())?;
        buf.data.copy_from_slice(secret);
        secret.zeroize();
        Ok(buf)
    }

    #[cfg(unix)]
    fn lock_pages(ptr: *mut u8, len: usize) -> Result<bool, MemoryError> {
        let res = unsafe { libc::mlock(ptr as *const libc::c_void, len) };
        if res == 0 {
            Ok(true)
        } else {
            Err(MemoryError::LockFailed(std::io::Error::last_os_error()))
        }
    }

    #[cfg(windows)]
    fn lock_pages(ptr: *mut u8, len: usize) -> Result<bool, MemoryError> {
        extern "system" {
            fn VirtualLock(lpAddress: *const core::ffi::c_void, dwSize: usize) -> i32;
        }
        let res = unsafe { VirtualLock(ptr as *const core::ffi::c_void, len) };
        if res != 0 {
            Ok(true)
        } else {
            Err(MemoryError::LockFailed(std::io::Error::last_os_error()))
        }
    }

    #[cfg(unix)]
    fn unlock_pages(ptr: *mut u8, len: usize) {
        unsafe {
            libc::munlock(ptr as *const libc::c_void, len);
        }
    }

    #[cfg(windows)]
    fn unlock_pages(ptr: *mut u8, len: usize) {
        extern "system" {
            fn VirtualUnlock(lpAddress: *const core::ffi::c_void, dwSize: usize) -> i32;
        }
        unsafe {
            VirtualUnlock(ptr as *const core::ffi::c_void, len);
        }
    }

}

impl Deref for LockedBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for LockedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Drop for LockedBuffer {
    fn drop(&mut self) {
        self.data.zeroize();
        if self.is_locked && !self.data.is_empty() {
            Self::unlock_pages(self.data.as_mut_ptr(), self.data.len());
        }
    }
}
