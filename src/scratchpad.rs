use rand::RngCore;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
pub enum ScratchpadError {
    #[error("I/O error during scratchpad operation: {0}")]
    Io(#[from] std::io::Error),
}

/// RAII guard for temporary files on disk/NVMe.
/// Overwrites physical blocks before unlinking on scope exit.
pub struct EphemeralFile {
    path: PathBuf,
    passes: u8,
}

impl EphemeralFile {
    pub fn new<P: AsRef<Path>>(path: P, overwrite_passes: u8) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            passes: overwrite_passes.max(1),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn shred_now(&self) -> Result<(), ScratchpadError> {
        if !self.path.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(&self.path)?;
        let len = metadata.len() as usize;

        if len > 0 {
            let mut file = OpenOptions::new().write(true).open(&self.path)?;
            let mut chunk = vec![0u8; 65_536.min(len)];
            let mut rng = rand::thread_rng();

            for pass in 0..self.passes {
                file.seek(SeekFrom::Start(0))?;
                let mut remaining = len;

                while remaining > 0 {
                    let to_write = remaining.min(chunk.len());
                    if pass == self.passes - 1 {
                        chunk[..to_write].zeroize();
                    } else {
                        rng.fill_bytes(&mut chunk[..to_write]);
                    }
                    file.write_all(&chunk[..to_write])?;
                    remaining -= to_write;
                }
                file.sync_all()?;
            }

            chunk.zeroize();
        }

        fs::remove_file(&self.path)?;
        Ok(())
    }
}

impl Drop for EphemeralFile {
    fn drop(&mut self) {
        let _ = self.shred_now();
    }
}
