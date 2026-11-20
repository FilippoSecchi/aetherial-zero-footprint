use crate::memory::LockedBuffer;
use crate::scratchpad::EphemeralFile;
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub struct NodeEphemeralBuffer {
    inner: Option<LockedBuffer>,
}

#[napi]
impl NodeEphemeralBuffer {
    #[napi(constructor)]
    pub fn new(mut secret: Buffer) -> Result<Self> {
        let locked = LockedBuffer::from_secret_slice(secret.as_mut())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self { inner: Some(locked) })
    }

    #[napi]
    pub fn read(&self) -> Result<Buffer> {
        let buf = self
            .inner
            .as_ref()
            .ok_or_else(|| Error::from_reason("Buffer already destroyed"))?;
        Ok(Buffer::from(buf.to_vec()))
    }

    #[napi]
    pub fn destroy(&mut self) {
        self.inner.take();
    }
}

#[napi]
pub fn shred_scratchpad_file(path: String, passes: Option<u8>) -> Result<()> {
    let guard = EphemeralFile::new(path, passes.unwrap_or(2));
    guard
        .shred_now()
        .map_err(|e| Error::from_reason(e.to_string()))
}
