use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicI32, Ordering};

use crate::testsupport::{
    ASSERT_FAIL, TEST_RESULT_FAILED, TEST_RESULT_NO_ASSERT, TEST_RESULT_PASSED,
};

static ASSERTS_FAILED: AtomicI32 = AtomicI32::new(0);
static ASSERTS_PASSED: AtomicI32 = AtomicI32::new(0);

unsafe fn log_assert_result(message: *const c_char, passed: bool) {
    let status = if passed { "Passed" } else { "Failed" };
    let text = CString::new(format!(
        "Assert '{}': {status}",
        std::ffi::CStr::from_ptr(message).to_string_lossy()
    ))
    .unwrap();
    if passed {
        crate::testsupport::log::SDLTest_LogFromBuffer(text.as_ptr());
    } else {
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(text.as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_AssertFromBuffer(
    assertCondition: c_int,
    assertDescription: *const c_char,
) {
    if SDLTest_AssertCheckFromBuffer(assertCondition, assertDescription) == ASSERT_FAIL {
        libc::abort();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_AssertCheckFromBuffer(
    assertCondition: c_int,
    assertDescription: *const c_char,
) -> c_int {
    if assertCondition == ASSERT_FAIL {
        ASSERTS_FAILED.fetch_add(1, Ordering::Relaxed);
        log_assert_result(assertDescription, false);
    } else {
        ASSERTS_PASSED.fetch_add(1, Ordering::Relaxed);
        log_assert_result(assertDescription, true);
    }
    assertCondition
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_AssertPassFromBuffer(assertDescription: *const c_char) {
    ASSERTS_PASSED.fetch_add(1, Ordering::Relaxed);
    log_assert_result(assertDescription, true);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ResetAssertSummary() {
    ASSERTS_PASSED.store(0, Ordering::Relaxed);
    ASSERTS_FAILED.store(0, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_LogAssertSummary() {
    let total = ASSERTS_PASSED.load(Ordering::Relaxed) + ASSERTS_FAILED.load(Ordering::Relaxed);
    let passed = ASSERTS_PASSED.load(Ordering::Relaxed);
    let failed = ASSERTS_FAILED.load(Ordering::Relaxed);
    let message = CString::new(format!(
        "Assert Summary: Total={total} Passed={passed} Failed={failed}"
    ))
    .unwrap();
    if failed == 0 {
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    } else {
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_AssertSummaryToTestResult() -> c_int {
    if ASSERTS_FAILED.load(Ordering::Relaxed) > 0 {
        TEST_RESULT_FAILED
    } else if ASSERTS_PASSED.load(Ordering::Relaxed) > 0 {
        TEST_RESULT_PASSED
    } else {
        TEST_RESULT_NO_ASSERT
    }
}
