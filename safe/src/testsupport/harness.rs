use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::abi::generated_types::{
    self as sdl, SDL_bool, SDL_bool_SDL_FALSE, SDL_bool_SDL_TRUE, Uint64,
};
use crate::testsupport::{
    maybe_uninit_zeroed, optional_c_string, SDLTest_Md5Context, SDLTest_RandomContext,
    SDLTest_TestCaseReference, SDLTest_TestSuiteReference, TEST_ABORTED, TEST_RESULT_FAILED,
    TEST_RESULT_NO_ASSERT, TEST_RESULT_PASSED, TEST_RESULT_SETUP_FAILURE, TEST_RESULT_SKIPPED,
    TEST_SKIPPED, TEST_STARTED,
};

const INVALID_NAME_FORMAT: &str = "(Invalid)";

fn clock_seconds() -> f32 {
    unsafe {
        let frequency = sdl::SDL_GetPerformanceFrequency();
        if frequency == 0 {
            0.0
        } else {
            sdl::SDL_GetPerformanceCounter() as f32 / frequency as f32
        }
    }
}

unsafe fn generate_exec_key(
    run_seed: &CStr,
    suite_name: &CStr,
    test_name: &CStr,
    iteration: i32,
) -> Uint64 {
    let mut ctx = maybe_uninit_zeroed::<SDLTest_Md5Context>().assume_init();
    let mut buffer = Vec::new();
    buffer.extend_from_slice(run_seed.to_bytes());
    buffer.extend_from_slice(suite_name.to_bytes());
    buffer.extend_from_slice(test_name.to_bytes());
    buffer.extend_from_slice(iteration.to_string().as_bytes());
    crate::testsupport::md5::SDLTest_Md5Init(&mut ctx);
    crate::testsupport::md5::SDLTest_Md5Update(&mut ctx, buffer.as_mut_ptr(), buffer.len() as u32);
    crate::testsupport::md5::SDLTest_Md5Final(&mut ctx);
    let mut key = [0u8; 8];
    key.copy_from_slice(&ctx.digest[..8]);
    u64::from_ne_bytes(key)
}

unsafe fn run_test(
    test_suite: *mut SDLTest_TestSuiteReference,
    test_case: *const SDLTest_TestCaseReference,
    exec_key: Uint64,
    force_test_run: SDL_bool,
) -> i32 {
    if test_suite.is_null() || test_case.is_null() {
        let message = CString::new("Setup failure: testSuite or testCase references NULL").unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
        return TEST_RESULT_SETUP_FAILURE;
    }
    if (*test_case).enabled == 0 && force_test_run == SDL_bool_SDL_FALSE {
        let name =
            optional_c_string((*test_case).name).unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
        let message = CString::new(format!(">>> Test '{}': Skipped (Disabled)\n", name)).unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
        return TEST_RESULT_SKIPPED;
    }

    crate::testsupport::fuzzer::SDLTest_FuzzerInit(exec_key);
    crate::testsupport::assert::SDLTest_ResetAssertSummary();

    if let Some(setup) = (*test_suite).testSetUp {
        setup(ptr::null_mut());
        if crate::testsupport::assert::SDLTest_AssertSummaryToTestResult() == TEST_RESULT_FAILED {
            let suite_name = optional_c_string((*test_suite).name)
                .unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
            let message =
                CString::new(format!(">>> Suite Setup '{}': Failed\n", suite_name)).unwrap();
            crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
            return TEST_RESULT_SETUP_FAILURE;
        }
    }

    let test_case_result = (*test_case)
        .testCase
        .map(|f| f(ptr::null_mut()))
        .unwrap_or(TEST_ABORTED);
    let test_result = if test_case_result == TEST_SKIPPED {
        TEST_RESULT_SKIPPED
    } else if test_case_result == TEST_STARTED || test_case_result == TEST_ABORTED {
        TEST_RESULT_FAILED
    } else {
        crate::testsupport::assert::SDLTest_AssertSummaryToTestResult()
    };

    if let Some(teardown) = (*test_suite).testTearDown {
        teardown(ptr::null_mut());
    }

    let fuzzer_count = crate::testsupport::fuzzer::SDLTest_GetFuzzerInvocationCount();
    if fuzzer_count > 0 {
        let message = CString::new(format!("Fuzzer invocations: {fuzzer_count}")).unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    }
    if test_case_result == TEST_SKIPPED {
        let name =
            optional_c_string((*test_case).name).unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
        let message =
            CString::new(format!(">>> Test '{}': Skipped (Programmatically)\n", name)).unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    } else if test_case_result == TEST_STARTED {
        let name =
            optional_c_string((*test_case).name).unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
        let message = CString::new(format!(
            ">>> Test '{}': Failed (test started, but did not return TEST_COMPLETED)\n",
            name
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
    } else if test_case_result == TEST_ABORTED {
        let name =
            optional_c_string((*test_case).name).unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
        let message = CString::new(format!(">>> Test '{}': Failed (Aborted)\n", name)).unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
    } else {
        crate::testsupport::assert::SDLTest_LogAssertSummary();
    }

    test_result
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_GenerateRunSeed(length: libc::c_int) -> *mut c_char {
    if length <= 0 {
        let message = CString::new("The length of the harness seed must be >0.").unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
        return ptr::null_mut();
    }
    let mut ctx = SDLTest_RandomContext {
        a: 0,
        x: 0,
        c: 0,
        ah: 0,
        al: 0,
    };
    crate::testsupport::random::SDLTest_RandomInitTime(&mut ctx);
    let mut output = String::with_capacity(length as usize);
    for _ in 0..length {
        let number = crate::testsupport::random::SDLTest_Random(&mut ctx);
        let mut ch = ((number % (91 - 48)) + 48) as u8;
        if (58..=64).contains(&ch) {
            ch = b'A';
        }
        output.push(ch as char);
    }
    crate::testsupport::alloc_c_string(&output)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RunSuites(
    testSuites: *mut *mut SDLTest_TestSuiteReference,
    userRunSeed: *const c_char,
    userExecKey: Uint64,
    filter: *const c_char,
    mut testIterations: libc::c_int,
) -> libc::c_int {
    if testIterations < 1 {
        testIterations = 1;
    }

    let mut generated_seed: *mut c_char = ptr::null_mut();
    let run_seed = if userRunSeed.is_null() || *userRunSeed == 0 {
        generated_seed = SDLTest_GenerateRunSeed(16);
        if generated_seed.is_null() {
            let message = CString::new("Generating a random seed failed").unwrap();
            crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
            return 2;
        }
        generated_seed
    } else {
        userRunSeed.cast_mut()
    };
    let run_seed_cstr = CStr::from_ptr(run_seed);
    let filter_value = optional_c_string(filter);

    let run_start_seconds = clock_seconds();
    let message = CString::new(format!(
        "::::: Test Run /w seed '{}' started\n",
        run_seed_cstr.to_string_lossy()
    ))
    .unwrap();
    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());

    let mut total_number_of_tests = 0;
    let mut suite_index = 0usize;
    while !(*testSuites.add(suite_index)).is_null() {
        let suite = *testSuites.add(suite_index);
        let mut case_index = 0usize;
        while !(*(*suite).testCases.add(case_index)).is_null() {
            total_number_of_tests += 1;
            case_index += 1;
        }
        suite_index += 1;
    }
    if total_number_of_tests == 0 {
        let message = CString::new("No tests to run?").unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
        if !generated_seed.is_null() && run_seed == generated_seed {
            sdl::SDL_free(generated_seed.cast());
        }
        return -1;
    }

    let mut suite_filter_name = None::<String>;
    let mut test_filter_name = None::<String>;
    if let Some(filter_name) = filter_value.as_ref() {
        let mut suite_counter = 0usize;
        while !(*testSuites.add(suite_counter)).is_null() && suite_filter_name.is_none() {
            let suite = *testSuites.add(suite_counter);
            if let Some(name) = optional_c_string((*suite).name) {
                if name.eq_ignore_ascii_case(filter_name) {
                    suite_filter_name = Some(name.clone());
                    let message =
                        CString::new(format!("Filtering: running only suite '{}'", filter_name))
                            .unwrap();
                    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                    break;
                }
            }
            let mut case_index = 0usize;
            while !(*(*suite).testCases.add(case_index)).is_null() && test_filter_name.is_none() {
                let test_case = *(*suite).testCases.add(case_index);
                if let Some(name) = optional_c_string((*test_case).name) {
                    if name.eq_ignore_ascii_case(filter_name) {
                        suite_filter_name = optional_c_string((*suite).name);
                        test_filter_name = Some(name.clone());
                        let message = CString::new(format!(
                            "Filtering: running only test '{}' in suite '{}'",
                            filter_name,
                            suite_filter_name.as_deref().unwrap_or(INVALID_NAME_FORMAT)
                        ))
                        .unwrap();
                        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                        break;
                    }
                }
                case_index += 1;
            }
            suite_counter += 1;
        }
        if suite_filter_name.is_none() && test_filter_name.is_none() {
            let message = CString::new(format!(
                "Filter '{}' did not match any test suite/case.",
                filter_name
            ))
            .unwrap();
            crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
            let message = CString::new("Exit code: 2").unwrap();
            crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
            if !generated_seed.is_null() && run_seed == generated_seed {
                sdl::SDL_free(generated_seed.cast());
            }
            return 2;
        }
    }

    let mut failed_tests: Vec<String> = Vec::new();
    let mut total_failed = 0;
    let mut total_passed = 0;
    let mut total_skipped = 0;
    let mut suite_counter = 0usize;
    while !(*testSuites.add(suite_counter)).is_null() {
        let suite = *testSuites.add(suite_counter);
        let current_suite_name =
            optional_c_string((*suite).name).unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
        suite_counter += 1;

        if let Some(ref suite_filter) = suite_filter_name {
            if optional_c_string((*suite).name)
                .map(|name| !name.eq_ignore_ascii_case(suite_filter))
                .unwrap_or(false)
            {
                let message = CString::new(format!(
                    "===== Test Suite {}: '{}' skipped\n",
                    suite_counter, current_suite_name
                ))
                .unwrap();
                crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                continue;
            }
        }

        let suite_start_seconds = clock_seconds();
        let message = CString::new(format!(
            "===== Test Suite {}: '{}' started\n",
            suite_counter, current_suite_name
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());

        let mut suite_failed = 0;
        let mut suite_passed = 0;
        let mut suite_skipped = 0;
        let mut case_index = 0usize;
        while !(*(*suite).testCases.add(case_index)).is_null() {
            let test_case = *(*suite).testCases.add(case_index);
            let current_test_name = optional_c_string((*test_case).name)
                .unwrap_or_else(|| INVALID_NAME_FORMAT.to_string());
            case_index += 1;
            if let Some(ref test_filter) = test_filter_name {
                if optional_c_string((*test_case).name)
                    .map(|name| !name.eq_ignore_ascii_case(test_filter))
                    .unwrap_or(false)
                {
                    let message = CString::new(format!(
                        "===== Test Case {}.{}: '{}' skipped\n",
                        suite_counter, case_index, current_test_name
                    ))
                    .unwrap();
                    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                    continue;
                }
            }

            let mut force_test_run = SDL_bool_SDL_FALSE;
            if test_filter_name.is_some() && (*test_case).enabled == 0 {
                force_test_run = SDL_bool_SDL_TRUE;
                let message =
                    CString::new("Force run of disabled test since test filter was set").unwrap();
                crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
            }

            let test_start_seconds = clock_seconds();
            let message = CString::new(format!(
                "----- Test Case {}.{}: '{}' started",
                suite_counter, case_index, current_test_name
            ))
            .unwrap();
            crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
            if let Some(description) = optional_c_string((*test_case).description) {
                if !description.is_empty() {
                    let message =
                        CString::new(format!("Test Description: '{}'", description)).unwrap();
                    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                }
            }

            let mut test_result = TEST_RESULT_FAILED;
            for iteration in 1..=testIterations {
                let exec_key = if userExecKey != 0 {
                    userExecKey
                } else {
                    let suite_name = CStr::from_ptr((*suite).name);
                    let test_name = CStr::from_ptr((*test_case).name);
                    generate_exec_key(run_seed_cstr, suite_name, test_name, iteration)
                };
                let message = CString::new(format!(
                    "Test Iteration {}: execKey {}",
                    iteration, exec_key
                ))
                .unwrap();
                crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                test_result = run_test(suite, test_case, exec_key, force_test_run);
                match test_result {
                    TEST_RESULT_PASSED => {
                        suite_passed += 1;
                        total_passed += 1;
                    }
                    TEST_RESULT_SKIPPED => {
                        suite_skipped += 1;
                        total_skipped += 1;
                    }
                    _ => {
                        suite_failed += 1;
                        total_failed += 1;
                    }
                }
            }

            let runtime = (clock_seconds() - test_start_seconds).max(0.0);
            let message = if testIterations > 1 {
                CString::new(format!(
                    "Runtime of {} iterations: {:.1} sec",
                    testIterations, runtime
                ))
                .unwrap()
            } else {
                CString::new(format!("Total Test runtime: {:.1} sec", runtime)).unwrap()
            };
            crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
            if testIterations > 1 {
                let message = CString::new(format!(
                    "Average Test runtime: {:.5} sec",
                    runtime / testIterations as f32
                ))
                .unwrap();
                crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
            }

            match test_result {
                TEST_RESULT_PASSED => {
                    let message =
                        CString::new(format!(">>> Test '{}': Passed\n", current_test_name))
                            .unwrap();
                    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
                }
                TEST_RESULT_FAILED => {
                    let message =
                        CString::new(format!(">>> Test '{}': Failed\n", current_test_name))
                            .unwrap();
                    crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
                    failed_tests.push(current_test_name.clone());
                }
                TEST_RESULT_NO_ASSERT => {
                    let message =
                        CString::new(format!(">>> Test '{}': No Asserts\n", current_test_name))
                            .unwrap();
                    crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
                }
                _ => {}
            }
        }

        let runtime = (clock_seconds() - suite_start_seconds).max(0.0);
        let message = CString::new(format!("Total Suite runtime: {:.1} sec", runtime)).unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
        let total = suite_passed + suite_failed + suite_skipped;
        if suite_failed == 0 {
            let message = CString::new(format!(
                "Suite Summary: Total={} Passed={} Failed={} Skipped={}",
                total, suite_passed, suite_failed, suite_skipped
            ))
            .unwrap();
            crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
            let message =
                CString::new(format!(">>> Suite '{}': Passed\n", current_suite_name)).unwrap();
            crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
        } else {
            let message = CString::new(format!(
                "Suite Summary: Total={} Passed={} Failed={} Skipped={}",
                total, suite_passed, suite_failed, suite_skipped
            ))
            .unwrap();
            crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
            let message =
                CString::new(format!(">>> Suite '{}': Failed\n", current_suite_name)).unwrap();
            crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
        }
    }

    let runtime = (clock_seconds() - run_start_seconds).max(0.0);
    let message = CString::new(format!("Total Run runtime: {:.1} sec", runtime)).unwrap();
    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    let total = total_passed + total_failed + total_skipped;
    let run_result = if total_failed == 0 { 0 } else { 1 };
    let summary = CString::new(format!(
        "Run Summary: Total={} Passed={} Failed={} Skipped={}",
        total, total_passed, total_failed, total_skipped
    ))
    .unwrap();
    if run_result == 0 {
        crate::testsupport::log::SDLTest_LogFromBuffer(summary.as_ptr());
        let message = CString::new(format!(
            ">>> Run /w seed '{}': Passed\n",
            run_seed_cstr.to_string_lossy()
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    } else {
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(summary.as_ptr());
        let message = CString::new(format!(
            ">>> Run /w seed '{}': Failed\n",
            run_seed_cstr.to_string_lossy()
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
    }

    if !failed_tests.is_empty() {
        let message = CString::new("Harness input to repro failures:").unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
        for test in failed_tests {
            let message = CString::new(format!(
                " --seed {} --filter {}",
                run_seed_cstr.to_string_lossy(),
                test
            ))
            .unwrap();
            crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
        }
    }
    let message = CString::new(format!("Exit code: {}", run_result)).unwrap();
    crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    if !generated_seed.is_null() && run_seed == generated_seed {
        sdl::SDL_free(generated_seed.cast());
    }
    run_result
}
