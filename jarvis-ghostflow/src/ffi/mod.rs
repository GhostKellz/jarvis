/// Foreign Function Interface bindings for Zig-based components
/// This module provides safe Rust wrappers around the Zig implementations
pub mod ghostllm;
pub mod zqlite;
pub mod zeke;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// FFI-related errors
#[derive(Error, Debug)]
pub enum FFIError {
    #[error("Library not found: {0}")]
    LibraryNotFound(String),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Zig runtime error: {0}")]
    ZigRuntimeError(String),
    
    #[error("Memory allocation failed")]
    MemoryAllocationFailed,
    
    #[error("String conversion failed")]
    StringConversionFailed,
    
    #[error("JSON serialization failed: {0}")]
    JsonSerializationFailed(#[from] serde_json::Error),
    
    #[error("Null pointer encountered")]
    NullPointer,
}

pub type FFIResult<T> = Result<T, FFIError>;

/// Common FFI utilities and helper functions
pub struct FFIUtils;

impl FFIUtils {
    /// Convert Rust string to C string
    pub fn rust_string_to_c(s: &str) -> FFIResult<CString> {
        CString::new(s).map_err(|_| FFIError::StringConversionFailed)
    }
    
    /// Convert C string to Rust string
    pub unsafe fn c_string_to_rust(c_str: *const c_char) -> FFIResult<String> {
        if c_str.is_null() {
            return Err(FFIError::NullPointer);
        }
        
        let c_str = CStr::from_ptr(c_str);
        c_str.to_str()
            .map(|s| s.to_owned())
            .map_err(|_| FFIError::StringConversionFailed)
    }
    
    /// Free C string allocated by Zig
    pub unsafe fn free_c_string(c_str: *mut c_char) {
        if !c_str.is_null() {
            // Note: This assumes Zig uses standard C allocator
            // In practice, we'd need to call a Zig-provided free function
            libc::free(c_str as *mut c_void);
        }
    }
    
    /// Convert Rust struct to JSON C string for Zig
    pub fn struct_to_json_c_string<T: Serialize>(data: &T) -> FFIResult<CString> {
        let json = serde_json::to_string(data)?;
        Self::rust_string_to_c(&json)
    }
    
    /// Convert JSON C string from Zig to Rust struct
    pub unsafe fn json_c_string_to_struct<T: for<'de> Deserialize<'de>>(
        c_str: *const c_char
    ) -> FFIResult<T> {
        let json_str = Self::c_string_to_rust(c_str)?;
        serde_json::from_str(&json_str).map_err(FFIError::JsonSerializationFailed)
    }
}

/// Common status codes returned by Zig functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FFIStatus {
    Success = 0,
    Error = 1,
    InvalidParameter = 2,
    MemoryError = 3,
    NotInitialized = 4,
    AlreadyInitialized = 5,
    NotFound = 6,
    Timeout = 7,
    NetworkError = 8,
    PermissionDenied = 9,
    Unknown = 999,
}

impl From<c_int> for FFIStatus {
    fn from(code: c_int) -> Self {
        match code {
            0 => FFIStatus::Success,
            1 => FFIStatus::Error,
            2 => FFIStatus::InvalidParameter,
            3 => FFIStatus::MemoryError,
            4 => FFIStatus::NotInitialized,
            5 => FFIStatus::AlreadyInitialized,
            6 => FFIStatus::NotFound,
            7 => FFIStatus::Timeout,
            8 => FFIStatus::NetworkError,
            9 => FFIStatus::PermissionDenied,
            _ => FFIStatus::Unknown,
        }
    }
}

impl FFIStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, FFIStatus::Success)
    }
    
    pub fn to_result(self) -> FFIResult<()> {
        match self {
            FFIStatus::Success => Ok(()),
            _ => Err(FFIError::ZigRuntimeError(format!("{:?}", self))),
        }
    }
}

/// Generic FFI handle for Zig objects
#[repr(C)]
pub struct FFIHandle {
    ptr: *mut c_void,
}

impl FFIHandle {
    pub fn new(ptr: *mut c_void) -> Self {
        Self { ptr }
    }
    
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
    
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }
}

impl Drop for FFIHandle {
    fn drop(&mut self) {
        // Note: In practice, each component should provide its own cleanup function
        if !self.ptr.is_null() {
            tracing::warn!("FFI handle dropped without explicit cleanup");
        }
    }
}

/// Common configuration structure for FFI components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFIConfig {
    pub library_path: String,
    pub enable_logging: bool,
    pub log_level: String,
    pub timeout_ms: u32,
    pub max_memory_mb: u32,
    pub thread_count: u32,
    pub custom_params: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for FFIConfig {
    fn default() -> Self {
        Self {
            library_path: String::new(),
            enable_logging: true,
            log_level: "info".to_string(),
            timeout_ms: 30000,
            max_memory_mb: 1024,
            thread_count: 4,
            custom_params: std::collections::HashMap::new(),
        }
    }
}

/// Trait for FFI component initialization and management
pub trait FFIComponent {
    type Config;
    type Handle;
    
    /// Initialize the FFI component
    fn initialize(config: Self::Config) -> FFIResult<Self::Handle>;
    
    /// Check if the component is initialized and healthy
    fn health_check(handle: &Self::Handle) -> FFIResult<bool>;
    
    /// Shutdown the component and cleanup resources
    fn shutdown(handle: Self::Handle) -> FFIResult<()>;
    
    /// Get component version information
    fn get_version() -> FFIResult<String>;
}

/// Macro to define FFI function bindings with error handling
#[macro_export]
macro_rules! ffi_fn {
    ($lib:expr, $fn_name:ident, $fn_type:ty) => {{
        use std::ffi::CStr;
        let symbol_name = CStr::from_bytes_with_nul(concat!(stringify!($fn_name), "\0").as_bytes())
            .map_err(|_| FFIError::StringConversionFailed)?;
            
        unsafe {
            $lib.get::<$fn_type>(symbol_name.to_bytes())
                .map_err(|_| FFIError::FunctionNotFound(stringify!($fn_name).to_string()))?
        }
    }};
}

/// Macro to wrap FFI calls with error handling
#[macro_export]
macro_rules! ffi_call {
    ($fn_call:expr) => {{
        let result = unsafe { $fn_call };
        let status = FFIStatus::from(result);
        status.to_result()
    }};
}

/// Async wrapper for FFI operations that might block
pub struct AsyncFFIWrapper;

impl AsyncFFIWrapper {
    /// Execute a potentially blocking FFI operation on a thread pool
    pub async fn execute_blocking<F, T>(operation: F) -> FFIResult<T>
    where
        F: FnOnce() -> FFIResult<T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(operation)
            .await
            .map_err(|_| FFIError::ZigRuntimeError("Async operation failed".to_string()))?
    }
    
    /// Execute FFI operation with timeout
    pub async fn execute_with_timeout<F, T>(
        operation: F,
        timeout: std::time::Duration,
    ) -> FFIResult<T>
    where
        F: FnOnce() -> FFIResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let future = Self::execute_blocking(operation);
        
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| FFIError::ZigRuntimeError("Operation timed out".to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffi_status_conversion() {
        assert_eq!(FFIStatus::from(0), FFIStatus::Success);
        assert_eq!(FFIStatus::from(1), FFIStatus::Error);
        assert_eq!(FFIStatus::from(999), FFIStatus::Unknown);
        assert_eq!(FFIStatus::from(-1), FFIStatus::Unknown);
    }
    
    #[test]
    fn test_string_conversion() {
        let rust_str = "test string";
        let c_string = FFIUtils::rust_string_to_c(rust_str).unwrap();
        
        unsafe {
            let converted_back = FFIUtils::c_string_to_rust(c_string.as_ptr()).unwrap();
            assert_eq!(rust_str, converted_back);
        }
    }
    
    #[tokio::test]
    async fn test_async_ffi_wrapper() {
        let result = AsyncFFIWrapper::execute_blocking(|| {
            // Simulate FFI operation
            Ok(42)
        }).await;
        
        assert_eq!(result.unwrap(), 42);
    }
}