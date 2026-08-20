// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Private native bridge for the experimental MoonBit binding.
//!
//! Error message pointers are borrowed until their result is freed. Result and
//! operator pointers must each be consumed or freed exactly once.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;
use std::str;
use std::sync::Mutex;

use opendal::Operator;
use opendal::services::Memory;

const STATUS_OK: i32 = 0;
const STATUS_ERROR: i32 = 1;
const UNAVAILABLE_RESULT: &str = "native result is unavailable";

pub struct NativeOperator(Mutex<Option<Operator>>);

impl NativeOperator {
    fn new(operator: Operator) -> Self {
        Self(Mutex::new(Some(operator)))
    }

    fn close(&self) {
        let operator = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        drop(operator);
    }

    #[cfg(test)]
    fn is_closed(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_none()
    }
}

pub struct NativeResult {
    error_message: Option<String>,
    operator: Option<Box<NativeOperator>>,
}

impl NativeResult {
    fn error(message: impl Into<String>) -> Self {
        Self {
            error_message: Some(message.into()),
            operator: None,
        }
    }

    fn operator(operator: NativeOperator) -> Self {
        Self {
            error_message: None,
            operator: Some(Box::new(operator)),
        }
    }

    fn status(&self) -> i32 {
        if self.error_message.is_some() {
            STATUS_ERROR
        } else {
            STATUS_OK
        }
    }

    fn error_message(&self) -> &str {
        self.error_message.as_deref().unwrap_or(UNAVAILABLE_RESULT)
    }
}

fn protect(operation: impl FnOnce() -> Result<NativeOperator, String>) -> *mut NativeResult {
    let result = match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(operator)) => NativeResult::operator(operator),
        Ok(Err(message)) => NativeResult::error(message),
        Err(_) => NativeResult::error("native binding panicked while creating an operator"),
    };
    Box::into_raw(Box::new(result))
}

unsafe fn copy_text(data: *const u8, len: u32, label: &str) -> Result<String, String> {
    if len != 0 && data.is_null() {
        return Err(format!("{label} pointer is null"));
    }
    let bytes = if len == 0 {
        &[]
    } else {
        // SAFETY: the caller guarantees that `data` points to `len` readable bytes.
        unsafe { slice::from_raw_parts(data, len as usize) }
    };
    str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| format!("{label} must be valid UTF-8"))
}

fn build_operator(scheme: &str) -> Result<NativeOperator, String> {
    if scheme != "memory" {
        return Err("only the memory service is enabled".to_string());
    }
    Operator::new(Memory::default())
        .map(NativeOperator::new)
        .map_err(|error| error.to_string())
}

/// # Safety
/// `scheme` must point to `scheme_len` readable bytes when `scheme_len` is nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_operator_new(
    scheme: *const u8,
    scheme_len: u32,
) -> *mut NativeResult {
    protect(|| {
        // SAFETY: the caller upholds this function's pointer contract.
        let scheme = unsafe { copy_text(scheme, scheme_len, "service scheme")? };
        build_operator(&scheme)
    })
}

/// # Safety
/// `operator` must be null or point to a live handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_operator_close(operator: *mut NativeOperator) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the caller provides either null or a live handle.
        if let Some(operator) = unsafe { operator.as_ref() } {
            operator.close();
        }
    }));
}

/// # Safety
/// `operator` must be null or an unfreed handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_operator_free(operator: *mut NativeOperator) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !operator.is_null() {
            // SAFETY: the caller transfers this allocation exactly once.
            drop(unsafe { Box::from_raw(operator) });
        }
    }));
}

/// # Safety
/// `result` must be null or point to a live result returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_result_status(result: *const NativeResult) -> i32 {
    // SAFETY: the caller provides either null or a live result.
    unsafe { result.as_ref() }
        .map(NativeResult::status)
        .unwrap_or(STATUS_ERROR)
}

/// # Safety
/// `result` must be null or point to a live result returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_result_error_message_data(
    result: *const NativeResult,
) -> *const u8 {
    // SAFETY: the caller provides either null or a live result.
    unsafe { result.as_ref() }
        .map(NativeResult::error_message)
        .unwrap_or(UNAVAILABLE_RESULT)
        .as_ptr()
}

/// # Safety
/// `result` must be null or point to a live result returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_result_error_message_len(
    result: *const NativeResult,
) -> usize {
    // SAFETY: the caller provides either null or a live result.
    unsafe { result.as_ref() }
        .map(NativeResult::error_message)
        .unwrap_or(UNAVAILABLE_RESULT)
        .len()
}

/// # Safety
/// `result` must be null or point to a live result returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_result_take_operator(
    result: *mut NativeResult,
) -> *mut NativeOperator {
    // SAFETY: the caller provides either null or a live result.
    unsafe { result.as_mut() }
        .and_then(|result| result.operator.take())
        .map(Box::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// `result` must be null or an unfreed result returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendal_moonbit_result_free(result: *mut NativeResult) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !result.is_null() {
            // SAFETY: the caller transfers this allocation exactly once.
            drop(unsafe { Box::from_raw(result) });
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_is_idempotent() {
        let operator = build_operator("memory").expect("memory operator must build");
        operator.close();
        operator.close();
        assert!(operator.is_closed());
    }

    #[test]
    fn unsupported_scheme_keeps_error_message() {
        let error = build_operator("unsupported")
            .err()
            .expect("scheme must be rejected");
        assert_eq!(error, "only the memory service is enabled");
    }

    #[test]
    fn panic_is_contained_as_an_error() {
        let result = protect(|| panic!("test panic"));
        // SAFETY: `protect` returns a live result owned by this test.
        let result_ref = unsafe { &*result };
        assert_eq!(result_ref.status(), STATUS_ERROR);
        assert_eq!(
            result_ref.error_message(),
            "native binding panicked while creating an operator"
        );
        // SAFETY: this test transfers its result exactly once.
        unsafe { opendal_moonbit_result_free(result) };
    }
}
