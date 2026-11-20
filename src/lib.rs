#[cfg(feature = "native")]
pub mod memory;
#[cfg(feature = "native")]
pub mod scratchpad;

pub mod bindings;

#[cfg(feature = "native")]
pub use memory::{LockedBuffer, MemoryError};
#[cfg(feature = "native")]
pub use scratchpad::{EphemeralFile, ScratchpadError};

#[cfg(feature = "native")]
pub fn with_ephemeral_memory<F, R>(size: usize, operation: F) -> Result<R, MemoryError>
where
    F: FnOnce(&mut [u8]) -> R,
{
    let mut buffer = LockedBuffer::new(size)?;
    let result = operation(&mut buffer);
    Ok(result)
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_locked_buffer_lifecycle() {
        let mut raw_secret = b"TOP_SECRET_TENANT_KEY_256BIT".to_vec();
        {
            let locked = LockedBuffer::from_secret_slice(&mut raw_secret).unwrap();
            assert_eq!(&*locked, b"TOP_SECRET_TENANT_KEY_256BIT");
            assert!(raw_secret.iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn test_ephemeral_file_shredding() {
        let temp_path = std::env::temp_dir().join("test_aetherial_scratch.tmp");
        {
            let guard = EphemeralFile::new(&temp_path, 2);
            let mut f = std::fs::File::create(guard.path()).unwrap();
            f.write_all(b"SENSITIVE_PAYLOAD_IN_TMP").unwrap();
            f.sync_all().unwrap();
            assert!(temp_path.exists());
        }
        assert!(!temp_path.exists());
    }
}


