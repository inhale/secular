// secular-core/src/ffi.rs
// UniFFI exports for Swift (iOS) and Android (Kotlin) bindings

use crate::config::SecularConfig;
use crate::protocol::SecularEngine;

// Used by full FFI implementation (UniFFI bindings)
#[allow(unused_imports)]
use crate::protocol::ConnectionState;
#[allow(unused_imports)]
use parking_lot::Mutex;
#[allow(unused_imports)]
use std::sync::Arc;

/// opaque handle to SecularEngine for FFI
pub struct SecularHandle {
    engine: SecularEngine,
}

/// Create a new Secular engine
///
/// # Safety
/// `config_json` must be a valid null-terminated UTF-8 string.
#[export_name = "secular_create"]
pub unsafe extern "C" fn secular_create(config_json: *const u8, len: usize) -> *mut SecularHandle {
    let config_str =
        std::str::from_utf8(std::slice::from_raw_parts(config_json, len)).unwrap_or("");
    let config: SecularConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    match SecularEngine::new(config) {
        Ok(engine) => Box::into_raw(Box::new(SecularHandle { engine })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Destroy a Secular engine handle
///
/// # Safety
/// `handle` must be a valid pointer returned by `secular_create`.
#[export_name = "secular_destroy"]
pub unsafe extern "C" fn secular_destroy(handle: *mut SecularHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get connection state as integer
/// 0 = Disconnected, 1 = Handshaking, 2 = Connected, 3 = Failed
///
/// # Safety
/// `handle` must be a valid pointer returned by `secular_create`.
#[export_name = "secular_state"]
pub unsafe extern "C" fn secular_state(handle: *const SecularHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    // Note: In async context, this would need to poll the state synchronously
    // For now return disconnected as placeholder
    0
}
