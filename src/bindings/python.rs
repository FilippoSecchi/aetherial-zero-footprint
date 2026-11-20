use crate::memory::LockedBuffer;
use crate::scratchpad::EphemeralFile;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

#[pyclass]
pub struct PyEphemeralBuffer {
    inner: Option<LockedBuffer>,
}

#[pymethods]
impl PyEphemeralBuffer {
    #[new]
    fn new(mut secret: Vec<u8>) -> PyResult<Self> {
        let locked = LockedBuffer::from_secret_slice(&mut secret)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: Some(locked) })
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) {
        self.inner.take();
    }

    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) {
        self.inner.take();
    }

    fn read_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
        let buf = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("Buffer has already been zeroized and dropped"))?;
        Ok(pyo3::types::PyBytes::new_bound(py, buf))
    }
}

#[pyfunction]
#[pyo3(signature = (path, passes=2))]
fn shred_file(path: String, passes: u8) -> PyResult<()> {
    let guard = EphemeralFile::new(path, passes);
    guard
        .shred_now()
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pymodule]
fn aetherial_zero_footprint(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEphemeralBuffer>()?;
    m.add_function(wrap_pyfunction!(shred_file, m)?)?;
    Ok(())
}
