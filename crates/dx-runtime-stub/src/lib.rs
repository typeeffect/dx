pub mod archive;
pub mod build_plan;
pub mod link;
pub mod manifest;
pub mod plan;

use std::ffi::{c_char, c_void, CStr};
use std::ptr;

pub type EnvHandle = *mut c_void;
pub type ClosureHandle = *mut c_void;
pub type PyObjHandle = *mut c_void;
pub type Utf8Ptr = *const c_char;

#[repr(C)]
struct StubClosure {
    code_ptr: *mut c_void,
    env: EnvHandle,
    arity: u32,
    capture_count: u32,
}

#[repr(C)]
struct Capture2I64 {
    a: i64,
    b: i64,
}

#[repr(C)]
struct Capture3I64 {
    a: i64,
    b: i64,
    c: i64,
}

#[repr(C)]
struct Capture2F64 {
    a: f64,
    b: f64,
}

#[repr(C)]
struct Capture3F64 {
    a: f64,
    b: f64,
    c: f64,
}

#[repr(C)]
struct Capture2I1 {
    a: bool,
    b: bool,
}

#[repr(C)]
struct Capture3I1 {
    a: bool,
    b: bool,
    c: bool,
}

#[repr(C)]
struct Capture2Ptr {
    a: *mut c_void,
    b: *mut c_void,
}

#[repr(C)]
struct Capture3Ptr {
    a: *mut c_void,
    b: *mut c_void,
    c: *mut c_void,
}

#[repr(C)]
struct CapturePtrI64 {
    p: *mut c_void,
    i: i64,
}

#[repr(C)]
struct CaptureI64PtrI1 {
    i: i64,
    p: *mut c_void,
    b: bool,
}

#[repr(C)]
struct StubTaggedValue {
    tag: Utf8Ptr,
}

fn closure_ptr(closure: ClosureHandle) -> Option<*const StubClosure> {
    if closure.is_null() {
        None
    } else {
        Some(closure as *const StubClosure)
    }
}

fn closure_code_ptr(closure: ClosureHandle) -> Option<(*const StubClosure, *mut c_void)> {
    let closure_ptr = closure_ptr(closure)?;
    let closure = unsafe { &*closure_ptr };
    if closure.code_ptr.is_null() {
        None
    } else {
        Some((closure_ptr, closure.code_ptr))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_create(
    code_ptr: *mut c_void,
    env: EnvHandle,
    arity: u32,
    capture_count: u32,
) -> ClosureHandle {
    let closure = Box::new(StubClosure {
        code_ptr,
        env,
        arity,
        capture_count,
    });
    Box::into_raw(closure) as ClosureHandle
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_i64_1_i64(closure: ClosureHandle, arg0: i64) -> i64 {
    let Some(closure) = closure_ptr(closure) else {
        return 0;
    };
    let closure = unsafe { &*closure };
    if closure.code_ptr.is_null() {
        return 0;
    }
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(closure.code_ptr) };
            fun(arg0)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const i64) };
            let fun: extern "C" fn(i64, i64) -> i64 =
                unsafe { std::mem::transmute(closure.code_ptr) };
            fun(capture0, arg0)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const Capture2I64) };
            let fun: extern "C" fn(i64, i64, i64) -> i64 =
                unsafe { std::mem::transmute(closure.code_ptr) };
            fun(env.a, env.b, arg0)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3I64) };
            let fun: extern "C" fn(i64, i64, i64, i64) -> i64 =
                unsafe { std::mem::transmute(closure.code_ptr) };
            fun(env.a, env.b, env.c, arg0)
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_f64_1_f64(closure: ClosureHandle, arg0: f64) -> f64 {
    let Some((closure_ptr, code_ptr)) = closure_code_ptr(closure) else {
        return 0.0;
    };
    let closure = unsafe { &*closure_ptr };
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(f64) -> f64 = unsafe { std::mem::transmute(code_ptr) };
            fun(arg0)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const f64) };
            let fun: extern "C" fn(f64, f64) -> f64 = unsafe { std::mem::transmute(code_ptr) };
            fun(capture0, arg0)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const Capture2F64) };
            let fun: extern "C" fn(f64, f64, f64) -> f64 =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, arg0)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3F64) };
            let fun: extern "C" fn(f64, f64, f64, f64) -> f64 =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, env.c, arg0)
        }
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_i1_1_i1(closure: ClosureHandle, arg0: bool) -> bool {
    let Some((closure_ptr, code_ptr)) = closure_code_ptr(closure) else {
        return false;
    };
    let closure = unsafe { &*closure_ptr };
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(bool) -> bool = unsafe { std::mem::transmute(code_ptr) };
            fun(arg0)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const bool) };
            let fun: extern "C" fn(bool, bool) -> bool = unsafe { std::mem::transmute(code_ptr) };
            fun(capture0, arg0)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const Capture2I1) };
            let fun: extern "C" fn(bool, bool, bool) -> bool =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, arg0)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3I1) };
            let fun: extern "C" fn(bool, bool, bool, bool) -> bool =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, env.c, arg0)
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_i64_2_i64_i64(
    closure: ClosureHandle,
    arg0: i64,
    arg1: i64,
) -> i64 {
    let Some((closure_ptr, code_ptr)) = closure_code_ptr(closure) else {
        return 0;
    };
    let closure = unsafe { &*closure_ptr };
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(i64, i64) -> i64 = unsafe { std::mem::transmute(code_ptr) };
            fun(arg0, arg1)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const i64) };
            let fun: extern "C" fn(i64, i64, i64) -> i64 =
                unsafe { std::mem::transmute(code_ptr) };
            fun(capture0, arg0, arg1)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const Capture2I64) };
            let fun: extern "C" fn(i64, i64, i64, i64) -> i64 =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, arg0, arg1)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3I64) };
            let fun: extern "C" fn(i64, i64, i64, i64, i64) -> i64 =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, env.c, arg0, arg1)
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_ptr_1_ptr(
    closure: ClosureHandle,
    arg0: *mut c_void,
) -> *mut c_void {
    let Some((closure_ptr, code_ptr)) = closure_code_ptr(closure) else {
        return ptr::null_mut();
    };
    let closure = unsafe { &*closure_ptr };
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(arg0)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const *mut c_void) };
            let fun: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(capture0, arg0)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const Capture2Ptr) };
            let fun: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, arg0)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3Ptr) };
            let fun: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, env.c, arg0)
        }
        _ => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_ptr_1_i64(
    closure: ClosureHandle,
    arg0: i64,
) -> *mut c_void {
    let Some((closure_ptr, code_ptr)) = closure_code_ptr(closure) else {
        return ptr::null_mut();
    };
    let closure = unsafe { &*closure_ptr };
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(i64) -> *mut c_void = unsafe { std::mem::transmute(code_ptr) };
            fun(arg0)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const *mut c_void) };
            let fun: extern "C" fn(*mut c_void, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(capture0, arg0)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const CapturePtrI64) };
            let fun: extern "C" fn(*mut c_void, i64, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.p, env.i, arg0)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3Ptr) };
            let fun: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, env.c, arg0)
        }
        _ => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_ptr_2_ptr_i64(
    closure: ClosureHandle,
    arg0: *mut c_void,
    arg1: i64,
) -> *mut c_void {
    let Some((closure_ptr, code_ptr)) = closure_code_ptr(closure) else {
        return ptr::null_mut();
    };
    let closure = unsafe { &*closure_ptr };
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(*mut c_void, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(arg0, arg1)
        }
        (false, 1) => {
            let capture0 = unsafe { *(closure.env as *const *mut c_void) };
            let fun: extern "C" fn(*mut c_void, *mut c_void, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(capture0, arg0, arg1)
        }
        (false, 2) => {
            let env = unsafe { &*(closure.env as *const CapturePtrI64) };
            let fun: extern "C" fn(*mut c_void, i64, *mut c_void, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.p, env.i, arg0, arg1)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const Capture3Ptr) };
            let fun: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, i64) -> *mut c_void =
                unsafe { std::mem::transmute(code_ptr) };
            fun(env.a, env.b, env.c, arg0, arg1)
        }
        _ => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_call_void_3_i64_ptr_i1(
    closure: ClosureHandle,
    arg0: i64,
    arg1: *mut c_void,
    arg2: bool,
) {
    let Some(closure_ptr) = closure_ptr(closure) else {
        return;
    };
    let closure = unsafe { &*closure_ptr };
    if closure.code_ptr.is_null() {
        return;
    }
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn(i64, *mut c_void, bool) =
                unsafe { std::mem::transmute(closure.code_ptr) };
            fun(arg0, arg1, arg2)
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const CaptureI64PtrI1) };
            let fun: extern "C" fn(i64, *mut c_void, bool, i64, *mut c_void, bool) =
                unsafe { std::mem::transmute(closure.code_ptr) };
            fun(env.i, env.p, env.b, arg0, arg1, arg2)
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_i64(closure: ClosureHandle) -> i64 {
    let Some(closure_ptr) = closure_ptr(closure) else {
        return 0;
    };
    let closure = unsafe { &*closure_ptr };
    if !closure.code_ptr.is_null() {
        return match (closure.env.is_null(), closure.capture_count) {
            (true, _) => {
                let fun: extern "C" fn() -> i64 = unsafe { std::mem::transmute(closure.code_ptr) };
                fun()
            }
            (false, 1) => {
                let capture0 = unsafe { *(closure.env as *const i64) };
                let fun: extern "C" fn(i64) -> i64 =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(capture0)
            }
            (false, 2) => {
                let env = unsafe { &*(closure.env as *const Capture2I64) };
                let fun: extern "C" fn(i64, i64) -> i64 =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b)
            }
            (false, 3) => {
                let env = unsafe { &*(closure.env as *const Capture3I64) };
                let fun: extern "C" fn(i64, i64, i64) -> i64 =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b, env.c)
            }
            _ => 0,
        };
    }
    if closure.env.is_null() {
        0
    } else {
        unsafe { *(closure.env as *const i64) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_f64(closure: ClosureHandle) -> f64 {
    let Some(closure_ptr) = closure_ptr(closure) else {
        return 0.0;
    };
    let closure = unsafe { &*closure_ptr };
    if !closure.code_ptr.is_null() {
        return match (closure.env.is_null(), closure.capture_count) {
            (true, _) => {
                let fun: extern "C" fn() -> f64 = unsafe { std::mem::transmute(closure.code_ptr) };
                fun()
            }
            (false, 1) => {
                let capture0 = unsafe { *(closure.env as *const f64) };
                let fun: extern "C" fn(f64) -> f64 =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(capture0)
            }
            (false, 2) => {
                let env = unsafe { &*(closure.env as *const Capture2F64) };
                let fun: extern "C" fn(f64, f64) -> f64 =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b)
            }
            (false, 3) => {
                let env = unsafe { &*(closure.env as *const Capture3F64) };
                let fun: extern "C" fn(f64, f64, f64) -> f64 =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b, env.c)
            }
            _ => 0.0,
        };
    }
    if closure.env.is_null() {
        0.0
    } else {
        unsafe { *(closure.env as *const f64) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_i1(closure: ClosureHandle) -> bool {
    let Some(closure_ptr) = closure_ptr(closure) else {
        return false;
    };
    let closure = unsafe { &*closure_ptr };
    if !closure.code_ptr.is_null() {
        return match (closure.env.is_null(), closure.capture_count) {
            (true, _) => {
                let fun: extern "C" fn() -> bool = unsafe { std::mem::transmute(closure.code_ptr) };
                fun()
            }
            (false, 1) => {
                let capture0 = unsafe { *(closure.env as *const bool) };
                let fun: extern "C" fn(bool) -> bool =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(capture0)
            }
            (false, 2) => {
                let env = unsafe { &*(closure.env as *const Capture2I1) };
                let fun: extern "C" fn(bool, bool) -> bool =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b)
            }
            (false, 3) => {
                let env = unsafe { &*(closure.env as *const Capture3I1) };
                let fun: extern "C" fn(bool, bool, bool) -> bool =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b, env.c)
            }
            _ => false,
        };
    }
    if closure.env.is_null() {
        false
    } else {
        unsafe { *(closure.env as *const bool) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_ptr(closure: ClosureHandle) -> *mut c_void {
    let Some(closure_ptr) = closure_ptr(closure) else {
        return ptr::null_mut();
    };
    let closure = unsafe { &*closure_ptr };
    if !closure.code_ptr.is_null() {
        return match (closure.env.is_null(), closure.capture_count) {
            (true, _) => {
                let fun: extern "C" fn() -> *mut c_void =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun()
            }
            (false, 1) => {
                let capture0 = unsafe { *(closure.env as *const *mut c_void) };
                let fun: extern "C" fn(*mut c_void) -> *mut c_void =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(capture0)
            }
            (false, 2) => {
                let env = unsafe { &*(closure.env as *const Capture2Ptr) };
                let fun: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b)
            }
            (false, 3) => {
                let env = unsafe { &*(closure.env as *const Capture3Ptr) };
                let fun: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                    unsafe { std::mem::transmute(closure.code_ptr) };
                fun(env.a, env.b, env.c)
            }
            _ => ptr::null_mut(),
        };
    }
    if closure.env.is_null() {
        ptr::null_mut()
    } else {
        unsafe { *(closure.env as *const *mut c_void) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_void(closure: ClosureHandle) {
    let Some(closure_ptr) = closure_ptr(closure) else {
        return;
    };
    let closure = unsafe { &*closure_ptr };
    if closure.code_ptr.is_null() {
        return;
    }
    match (closure.env.is_null(), closure.capture_count) {
        (true, _) => {
            let fun: extern "C" fn() = unsafe { std::mem::transmute(closure.code_ptr) };
            fun()
        }
        (false, 3) => {
            let env = unsafe { &*(closure.env as *const CaptureI64PtrI1) };
            let fun: extern "C" fn(i64, *mut c_void, bool) =
                unsafe { std::mem::transmute(closure.code_ptr) };
            fun(env.i, env.p, env.b)
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_match_tag(value_handle: *mut c_void, pattern_tag_name: Utf8Ptr) -> bool {
    if value_handle.is_null() || pattern_tag_name.is_null() {
        return false;
    }

    let value = unsafe { &*(value_handle as *const StubTaggedValue) };
    if value.tag.is_null() {
        return false;
    }

    let lhs = unsafe { CStr::from_ptr(value.tag) };
    let rhs = unsafe { CStr::from_ptr(pattern_tag_name) };
    lhs == rhs
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_throw_check_pending() {}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_py_call_function(_name: Utf8Ptr, _argc: u32) -> PyObjHandle {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_py_call_method(
    _receiver: PyObjHandle,
    _method_name: Utf8Ptr,
    _argc: u32,
) -> PyObjHandle {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_py_call_dynamic(_callable: PyObjHandle, _argc: u32) -> PyObjHandle {
    ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn free_closure(handle: ClosureHandle) {
        if !handle.is_null() {
            unsafe {
                drop(Box::from_raw(handle as *mut StubClosure));
            }
        }
    }

    fn free_env<T>(env: EnvHandle) {
        if !env.is_null() {
            unsafe {
                drop(Box::from_raw(env as *mut T));
            }
        }
    }

    fn tagged_value(tag: &CString) -> *mut c_void {
        Box::into_raw(Box::new(StubTaggedValue { tag: tag.as_ptr() })) as *mut c_void
    }

    fn free_tagged_value(value: *mut c_void) {
        if !value.is_null() {
            unsafe {
                drop(Box::from_raw(value as *mut StubTaggedValue));
            }
        }
    }

    #[test]
    fn closure_create_returns_handle() {
        let handle = dx_rt_closure_create(ptr::null_mut(), ptr::null_mut(), 0, 0);
        assert!(!handle.is_null());
        free_closure(handle);
    }

    #[test]
    fn thunk_calls_return_default_values() {
        let handle = dx_rt_closure_create(ptr::null_mut(), ptr::null_mut(), 0, 0);
        assert_eq!(dx_rt_thunk_call_i64(handle), 0);
        assert_eq!(dx_rt_thunk_call_f64(handle), 0.0);
        assert!(!dx_rt_thunk_call_i1(handle));
        assert!(dx_rt_thunk_call_ptr(handle).is_null());
        dx_rt_thunk_call_void(handle);
        free_closure(handle);
    }

    #[test]
    fn thunk_calls_can_read_captured_env_values() {
        let env_i64 = Box::into_raw(Box::new(42_i64)) as EnvHandle;
        let env_f64 = Box::into_raw(Box::new(3.5_f64)) as EnvHandle;
        let env_i1 = Box::into_raw(Box::new(true)) as EnvHandle;
        let payload = Box::into_raw(Box::new(123_i64)) as *mut c_void;
        let env_ptr = Box::into_raw(Box::new(payload)) as EnvHandle;

        let thunk_i64 = dx_rt_closure_create(ptr::null_mut(), env_i64, 0, 1);
        let thunk_f64 = dx_rt_closure_create(ptr::null_mut(), env_f64, 0, 1);
        let thunk_i1 = dx_rt_closure_create(ptr::null_mut(), env_i1, 0, 1);
        let thunk_ptr = dx_rt_closure_create(ptr::null_mut(), env_ptr, 0, 1);

        assert_eq!(dx_rt_thunk_call_i64(thunk_i64), 42);
        assert_eq!(dx_rt_thunk_call_f64(thunk_f64), 3.5);
        assert!(dx_rt_thunk_call_i1(thunk_i1));
        assert_eq!(dx_rt_thunk_call_ptr(thunk_ptr), payload);

        free_closure(thunk_i64);
        free_closure(thunk_f64);
        free_closure(thunk_i1);
        free_closure(thunk_ptr);
        free_env::<i64>(env_i64);
        free_env::<f64>(env_f64);
        free_env::<bool>(env_i1);
        free_env::<*mut c_void>(env_ptr);
        unsafe {
            drop(Box::from_raw(payload as *mut i64));
        }
    }

    #[test]
    fn closure_call_stubs_return_default_values() {
        let handle = dx_rt_closure_create(ptr::null_mut(), ptr::null_mut(), 1, 0);
        assert_eq!(dx_rt_closure_call_i64_1_i64(handle, 7), 0);
        assert_eq!(dx_rt_closure_call_i64_2_i64_i64(handle, 7, 9), 0);
        assert!(dx_rt_closure_call_ptr_1_ptr(handle, ptr::null_mut()).is_null());
        assert!(dx_rt_closure_call_ptr_1_i64(handle, 7).is_null());
        assert!(dx_rt_closure_call_ptr_2_ptr_i64(handle, ptr::null_mut(), 9).is_null());
        dx_rt_closure_call_void_3_i64_ptr_i1(handle, 1, ptr::null_mut(), false);
        free_closure(handle);
    }

    extern "C" fn plus_one(x: i64) -> i64 {
        x + 1
    }

    extern "C" fn forty_two() -> i64 {
        42
    }

    extern "C" fn add_captured(capture: i64, arg: i64) -> i64 {
        capture + arg
    }

    extern "C" fn add_pair(x: i64, y: i64) -> i64 {
        x + y
    }

    extern "C" fn add_pair_with_capture(capture: i64, x: i64, y: i64) -> i64 {
        capture + x + y
    }

    extern "C" fn echo_f64(x: f64) -> f64 {
        x
    }

    extern "C" fn add_f64_capture(capture: f64, x: f64) -> f64 {
        capture + x
    }

    extern "C" fn add_two_f64_captures(c0: f64, c1: f64, x: f64) -> f64 {
        c0 + c1 + x
    }

    extern "C" fn sum_two_f64_captures(c0: f64, c1: f64) -> f64 {
        c0 + c1
    }

    extern "C" fn add_three_f64_captures(c0: f64, c1: f64, c2: f64, x: f64) -> f64 {
        c0 + c1 + c2 + x
    }

    extern "C" fn sum_three_f64_captures(c0: f64, c1: f64, c2: f64) -> f64 {
        c0 + c1 + c2
    }

    extern "C" fn echo_i1(x: bool) -> bool {
        x
    }

    extern "C" fn xor_i1_capture(capture: bool, x: bool) -> bool {
        capture ^ x
    }

    extern "C" fn xor_two_i1_captures(c0: bool, c1: bool, x: bool) -> bool {
        c0 ^ c1 ^ x
    }

    extern "C" fn xor_two_i1_thunk_captures(c0: bool, c1: bool) -> bool {
        c0 ^ c1
    }

    extern "C" fn xor_three_i1_captures(c0: bool, c1: bool, c2: bool, x: bool) -> bool {
        c0 ^ c1 ^ c2 ^ x
    }

    extern "C" fn xor_three_i1_thunk_captures(c0: bool, c1: bool, c2: bool) -> bool {
        c0 ^ c1 ^ c2
    }

    extern "C" fn echo_ptr(x: *mut c_void) -> *mut c_void {
        x
    }

    extern "C" fn int_to_ptr(x: i64) -> *mut c_void {
        x as usize as *mut c_void
    }

    extern "C" fn prepend_capture_ptr(capture: *mut c_void, _arg: i64) -> *mut c_void {
        capture
    }

    extern "C" fn second_ptr_with_arg(
        _c0: *mut c_void,
        c1: *mut c_void,
        _arg: i64,
    ) -> *mut c_void {
        c1
    }

    extern "C" fn first_ptr_with_ptr_arg(
        c0: *mut c_void,
        _c1: *mut c_void,
        _arg: *mut c_void,
    ) -> *mut c_void {
        c0
    }

    extern "C" fn third_ptr_with_ptr_arg(
        _c0: *mut c_void,
        _c1: *mut c_void,
        c2: *mut c_void,
        _arg: *mut c_void,
    ) -> *mut c_void {
        c2
    }

    extern "C" fn third_ptr_with_i64_arg(
        _c0: *mut c_void,
        _c1: *mut c_void,
        c2: *mut c_void,
        _arg: i64,
    ) -> *mut c_void {
        c2
    }

    extern "C" fn third_ptr_with_mixed_args(
        _c0: *mut c_void,
        _c1: *mut c_void,
        c2: *mut c_void,
        _arg0: *mut c_void,
        _arg1: i64,
    ) -> *mut c_void {
        c2
    }

    extern "C" fn second_ptr_with_mixed_args(
        _c0: *mut c_void,
        c1: i64,
        _arg0: *mut c_void,
        _arg1: i64,
    ) -> *mut c_void {
        c1 as usize as *mut c_void
    }

    extern "C" fn ptr_from_mixed_capture(c0: *mut c_void, c1: i64, _arg: i64) -> *mut c_void {
        if c1 == 7 {
            c0
        } else {
            ptr::null_mut()
        }
    }

    extern "C" fn consume_with_mixed_capture(
        _c0: i64,
        c1: *mut c_void,
        c2: bool,
        arg0: i64,
        arg1: *mut c_void,
        arg2: bool,
    ) {
        assert_eq!(arg0, 1);
        assert_eq!(arg1, c1);
        assert_eq!(arg2, c2);
    }

    extern "C" fn ptr_identity_thunk(capture: *mut c_void) -> *mut c_void {
        capture
    }

    extern "C" fn second_ptr_thunk(_c0: *mut c_void, c1: *mut c_void) -> *mut c_void {
        c1
    }

    extern "C" fn third_ptr_thunk(
        _c0: *mut c_void,
        _c1: *mut c_void,
        c2: *mut c_void,
    ) -> *mut c_void {
        c2
    }

    extern "C" fn mixed_ptr_thunk(_c0: i64, c1: *mut c_void, c2: bool) -> *mut c_void {
        if c2 {
            c1
        } else {
            ptr::null_mut()
        }
    }

    static VOID_THUNK_CALLED: AtomicBool = AtomicBool::new(false);

    extern "C" fn mark_void_thunk() {
        VOID_THUNK_CALLED.store(true, Ordering::SeqCst);
    }

    extern "C" fn mixed_void_thunk(c0: i64, c1: *mut c_void, c2: bool) {
        assert_eq!(c0, 99);
        assert!(!c1.is_null());
        assert!(c2);
        VOID_THUNK_CALLED.store(true, Ordering::SeqCst);
    }

    #[test]
    fn ordinary_closure_calls_dispatch_through_code_ptr_without_env() {
        let c1 = dx_rt_closure_create(plus_one as *mut c_void, ptr::null_mut(), 1, 0);
        let c2 = dx_rt_closure_create(add_pair as *mut c_void, ptr::null_mut(), 2, 0);
        let c3 = dx_rt_closure_create(echo_ptr as *mut c_void, ptr::null_mut(), 1, 0);
        let c4 = dx_rt_closure_create(int_to_ptr as *mut c_void, ptr::null_mut(), 1, 0);
        let c5 = dx_rt_closure_create(echo_f64 as *mut c_void, ptr::null_mut(), 1, 0);
        let c6 = dx_rt_closure_create(echo_i1 as *mut c_void, ptr::null_mut(), 1, 0);
        let payload = 0x55usize as *mut c_void;

        assert_eq!(dx_rt_closure_call_i64_1_i64(c1, 41), 42);
        assert_eq!(dx_rt_closure_call_i64_2_i64_i64(c2, 20, 22), 42);
        assert_eq!(dx_rt_closure_call_ptr_1_ptr(c3, payload), payload);
        assert_eq!(dx_rt_closure_call_ptr_1_i64(c4, 42), 42usize as *mut c_void);
        assert_eq!(dx_rt_closure_call_f64_1_f64(c5, 3.5), 3.5);
        assert!(dx_rt_closure_call_i1_1_i1(c6, true));

        free_closure(c1);
        free_closure(c2);
        free_closure(c3);
        free_closure(c4);
        free_closure(c5);
        free_closure(c6);
    }

    #[test]
    fn ordinary_closure_i64_call_can_dispatch_with_single_i64_capture() {
        let env = Box::into_raw(Box::new(41_i64)) as EnvHandle;
        let closure = dx_rt_closure_create(add_captured as *mut c_void, env, 1, 1);

        assert_eq!(dx_rt_closure_call_i64_1_i64(closure, 1), 42);

        free_closure(closure);
        free_env::<i64>(env);
    }

    #[test]
    fn ordinary_closure_i64_pair_call_can_dispatch_with_single_i64_capture() {
        let env = Box::into_raw(Box::new(40_i64)) as EnvHandle;
        let closure = dx_rt_closure_create(add_pair_with_capture as *mut c_void, env, 2, 1);

        assert_eq!(dx_rt_closure_call_i64_2_i64_i64(closure, 1, 1), 42);

        free_closure(closure);
        free_env::<i64>(env);
    }

    #[test]
    fn ordinary_closure_ptr_return_can_dispatch_with_single_ptr_capture() {
        let payload = 0x1234usize as *mut c_void;
        let env = Box::into_raw(Box::new(payload)) as EnvHandle;
        let closure = dx_rt_closure_create(prepend_capture_ptr as *mut c_void, env, 1, 1);

        assert_eq!(dx_rt_closure_call_ptr_1_i64(closure, 7), payload);

        free_closure(closure);
        free_env::<*mut c_void>(env);
    }

    #[test]
    fn ordinary_closure_ptr_return_can_dispatch_with_two_ptr_captures() {
        let payload0 = 0x1234usize as *mut c_void;
        let payload1 = 0x5678usize as *mut c_void;
        let env = Box::into_raw(Box::new(Capture2Ptr {
            a: payload0,
            b: payload1,
        })) as EnvHandle;

        let c1 = dx_rt_closure_create(second_ptr_with_arg as *mut c_void, env, 1, 2);
        let c2 = dx_rt_closure_create(first_ptr_with_ptr_arg as *mut c_void, env, 1, 2);
        let c3 = dx_rt_closure_create(second_ptr_with_mixed_args as *mut c_void, env, 2, 2);

        assert_eq!(dx_rt_closure_call_ptr_1_i64(c1, 7), payload1);
        assert_eq!(dx_rt_closure_call_ptr_1_ptr(c2, 0x9999usize as *mut c_void), payload0);
        assert_eq!(
            dx_rt_closure_call_ptr_2_ptr_i64(c3, 0xaaaausize as *mut c_void, 9),
            payload1
        );

        free_closure(c1);
        free_closure(c2);
        free_closure(c3);
        free_env::<Capture2Ptr>(env);
    }

    #[test]
    fn ordinary_closure_ptr_return_can_dispatch_with_three_ptr_captures() {
        let payload0 = 0x1234usize as *mut c_void;
        let payload1 = 0x5678usize as *mut c_void;
        let payload2 = 0x9abcusize as *mut c_void;
        let env = Box::into_raw(Box::new(Capture3Ptr {
            a: payload0,
            b: payload1,
            c: payload2,
        })) as EnvHandle;

        let closure = dx_rt_closure_create(third_ptr_with_ptr_arg as *mut c_void, env, 1, 3);

        assert_eq!(
            dx_rt_closure_call_ptr_1_ptr(closure, 0xffffusize as *mut c_void),
            payload2
        );

        free_closure(closure);
        free_env::<Capture3Ptr>(env);
    }

    #[test]
    fn ordinary_closure_ptr_i64_and_ptr_ptr_i64_can_dispatch_with_three_ptr_captures() {
        let payload0 = 0x1234usize as *mut c_void;
        let payload1 = 0x5678usize as *mut c_void;
        let payload2 = 0x9abcusize as *mut c_void;
        let env = Box::into_raw(Box::new(Capture3Ptr {
            a: payload0,
            b: payload1,
            c: payload2,
        })) as EnvHandle;

        let c1 = dx_rt_closure_create(third_ptr_with_i64_arg as *mut c_void, env, 1, 3);
        let c2 = dx_rt_closure_create(third_ptr_with_mixed_args as *mut c_void, env, 2, 3);

        assert_eq!(dx_rt_closure_call_ptr_1_i64(c1, 7), payload2);
        assert_eq!(
            dx_rt_closure_call_ptr_2_ptr_i64(c2, 0xffffusize as *mut c_void, 9),
            payload2
        );

        free_closure(c1);
        free_closure(c2);
        free_env::<Capture3Ptr>(env);
    }

    #[test]
    fn ordinary_closure_ptr_return_can_dispatch_with_mixed_ptr_i64_captures() {
        let payload = 0x1234usize as *mut c_void;
        let env = Box::into_raw(Box::new(CapturePtrI64 { p: payload, i: 7 })) as EnvHandle;

        let c1 = dx_rt_closure_create(ptr_from_mixed_capture as *mut c_void, env, 1, 2);
        let c2 = dx_rt_closure_create(second_ptr_with_mixed_args as *mut c_void, env, 2, 2);

        assert_eq!(dx_rt_closure_call_ptr_1_i64(c1, 9), payload);
        assert_eq!(
            dx_rt_closure_call_ptr_2_ptr_i64(c2, 0xaaaausize as *mut c_void, 9),
            7usize as *mut c_void
        );

        free_closure(c1);
        free_closure(c2);
        free_env::<CapturePtrI64>(env);
    }

    #[test]
    fn ordinary_closure_f64_and_i1_calls_can_dispatch_with_single_capture() {
        let env_f64 = Box::into_raw(Box::new(38.5_f64)) as EnvHandle;
        let env_i1 = Box::into_raw(Box::new(true)) as EnvHandle;
        let closure_f64 = dx_rt_closure_create(add_f64_capture as *mut c_void, env_f64, 1, 1);
        let closure_i1 = dx_rt_closure_create(xor_i1_capture as *mut c_void, env_i1, 1, 1);

        assert_eq!(dx_rt_closure_call_f64_1_f64(closure_f64, 3.5), 42.0);
        assert!(!dx_rt_closure_call_i1_1_i1(closure_i1, true));

        free_closure(closure_f64);
        free_closure(closure_i1);
        free_env::<f64>(env_f64);
        free_env::<bool>(env_i1);
    }

    #[test]
    fn ordinary_closure_f64_and_i1_calls_can_dispatch_with_two_captures() {
        let env_f64 = Box::into_raw(Box::new(Capture2F64 { a: 20.0, b: 18.5 })) as EnvHandle;
        let env_i1 = Box::into_raw(Box::new(Capture2I1 { a: true, b: false })) as EnvHandle;
        let closure_f64 = dx_rt_closure_create(add_two_f64_captures as *mut c_void, env_f64, 1, 2);
        let closure_i1 = dx_rt_closure_create(xor_two_i1_captures as *mut c_void, env_i1, 1, 2);

        assert_eq!(dx_rt_closure_call_f64_1_f64(closure_f64, 3.5), 42.0);
        assert!(!dx_rt_closure_call_i1_1_i1(closure_i1, true));

        free_closure(closure_f64);
        free_closure(closure_i1);
        free_env::<Capture2F64>(env_f64);
        free_env::<Capture2I1>(env_i1);
    }

    #[test]
    fn ordinary_closure_f64_and_i1_calls_can_dispatch_with_three_captures() {
        let env_f64 = Box::into_raw(Box::new(Capture3F64 {
            a: 10.0,
            b: 12.5,
            c: 16.0,
        })) as EnvHandle;
        let env_i1 = Box::into_raw(Box::new(Capture3I1 {
            a: true,
            b: false,
            c: true,
        })) as EnvHandle;
        let closure_f64 =
            dx_rt_closure_create(add_three_f64_captures as *mut c_void, env_f64, 1, 3);
        let closure_i1 =
            dx_rt_closure_create(xor_three_i1_captures as *mut c_void, env_i1, 1, 3);

        assert_eq!(dx_rt_closure_call_f64_1_f64(closure_f64, 3.5), 42.0);
        assert!(dx_rt_closure_call_i1_1_i1(closure_i1, true));

        free_closure(closure_f64);
        free_closure(closure_i1);
        free_env::<Capture3F64>(env_f64);
        free_env::<Capture3I1>(env_i1);
    }

    #[test]
    fn closure_create_preserves_code_pointer() {
        let sentinel = 0x1234usize as *mut c_void;
        let handle = dx_rt_closure_create(sentinel, ptr::null_mut(), 1, 0);
        let closure = unsafe { &*(handle as *const StubClosure) };
        assert_eq!(closure.code_ptr, sentinel);
        assert_eq!(closure.capture_count, 0);
        free_closure(handle);
    }

    extern "C" fn add_two_captures(c0: i64, c1: i64, arg: i64) -> i64 {
        c0 + c1 + arg
    }

    extern "C" fn sum_two_i64_captures(c0: i64, c1: i64) -> i64 {
        c0 + c1
    }

    extern "C" fn add_pair_with_two_captures(c0: i64, c1: i64, x: i64, y: i64) -> i64 {
        c0 + c1 + x + y
    }

    extern "C" fn add_three_captures(c0: i64, c1: i64, c2: i64, arg: i64) -> i64 {
        c0 + c1 + c2 + arg
    }

    extern "C" fn add_pair_with_three_captures(c0: i64, c1: i64, c2: i64, x: i64, y: i64) -> i64 {
        c0 + c1 + c2 + x + y
    }

    extern "C" fn sum_three_i64_captures(c0: i64, c1: i64, c2: i64) -> i64 {
        c0 + c1 + c2
    }

    #[test]
    fn ordinary_closure_i64_call_can_dispatch_with_two_i64_captures() {
        let env = Box::into_raw(Box::new(Capture2I64 { a: 20, b: 21 })) as EnvHandle;
        let closure = dx_rt_closure_create(add_two_captures as *mut c_void, env, 1, 2);

        assert_eq!(dx_rt_closure_call_i64_1_i64(closure, 1), 42);

        free_closure(closure);
        free_env::<Capture2I64>(env);
    }

    #[test]
    fn ordinary_closure_i64_pair_call_can_dispatch_with_two_i64_captures() {
        let env = Box::into_raw(Box::new(Capture2I64 { a: 19, b: 20 })) as EnvHandle;
        let closure = dx_rt_closure_create(add_pair_with_two_captures as *mut c_void, env, 2, 2);

        assert_eq!(dx_rt_closure_call_i64_2_i64_i64(closure, 1, 2), 42);

        free_closure(closure);
        free_env::<Capture2I64>(env);
    }

    #[test]
    fn ordinary_closure_i64_call_can_dispatch_with_three_i64_captures() {
        let env = Box::into_raw(Box::new(Capture3I64 {
            a: 10,
            b: 11,
            c: 20,
        })) as EnvHandle;
        let closure = dx_rt_closure_create(add_three_captures as *mut c_void, env, 1, 3);

        assert_eq!(dx_rt_closure_call_i64_1_i64(closure, 1), 42);

        free_closure(closure);
        free_env::<Capture3I64>(env);
    }

    #[test]
    fn ordinary_closure_i64_pair_call_can_dispatch_with_three_i64_captures() {
        let env = Box::into_raw(Box::new(Capture3I64 {
            a: 10,
            b: 10,
            c: 19,
        })) as EnvHandle;
        let closure = dx_rt_closure_create(add_pair_with_three_captures as *mut c_void, env, 2, 3);

        assert_eq!(dx_rt_closure_call_i64_2_i64_i64(closure, 1, 2), 42);

        free_closure(closure);
        free_env::<Capture3I64>(env);
    }

    #[test]
    fn thunk_calls_can_dispatch_through_code_ptr_with_zero_one_or_two_captures() {
        let env_i64 = Box::into_raw(Box::new(Capture2I64 { a: 20, b: 22 })) as EnvHandle;
        let env_f64 = Box::into_raw(Box::new(Capture2F64 { a: 20.0, b: 22.0 })) as EnvHandle;
        let env_i1 = Box::into_raw(Box::new(Capture2I1 { a: true, b: false })) as EnvHandle;
        let payload = 0x1234usize as *mut c_void;
        let env_ptr = Box::into_raw(Box::new(payload)) as EnvHandle;

        let thunk0 = dx_rt_closure_create(forty_two as *mut c_void, ptr::null_mut(), 0, 0);
        let thunk_i64 = dx_rt_closure_create(sum_two_i64_captures as *mut c_void, env_i64, 0, 2);
        let thunk_f64 = dx_rt_closure_create(sum_two_f64_captures as *mut c_void, env_f64, 0, 2);
        let thunk_i1 =
            dx_rt_closure_create(xor_two_i1_thunk_captures as *mut c_void, env_i1, 0, 2);
        let thunk_ptr = dx_rt_closure_create(ptr_identity_thunk as *mut c_void, env_ptr, 0, 1);

        assert_eq!(dx_rt_thunk_call_i64(thunk0), 42);
        assert_eq!(dx_rt_thunk_call_i64(thunk_i64), 42);
        assert_eq!(dx_rt_thunk_call_f64(thunk_f64), 42.0);
        assert!(dx_rt_thunk_call_i1(thunk_i1));
        assert_eq!(dx_rt_thunk_call_ptr(thunk_ptr), payload);

        free_closure(thunk0);
        free_closure(thunk_i64);
        free_closure(thunk_f64);
        free_closure(thunk_i1);
        free_closure(thunk_ptr);
        free_env::<Capture2I64>(env_i64);
        free_env::<Capture2F64>(env_f64);
        free_env::<Capture2I1>(env_i1);
        free_env::<*mut c_void>(env_ptr);
    }

    #[test]
    fn thunk_ptr_calls_can_dispatch_with_two_ptr_or_mixed_captures() {
        let payload0 = 0x1234usize as *mut c_void;
        let payload1 = 0x5678usize as *mut c_void;
        let env_ptrs = Box::into_raw(Box::new(Capture2Ptr {
            a: payload0,
            b: payload1,
        })) as EnvHandle;
        let env_mixed = Box::into_raw(Box::new(CaptureI64PtrI1 {
            i: 99,
            p: payload1,
            b: true,
        })) as EnvHandle;

        let thunk_ptrs = dx_rt_closure_create(second_ptr_thunk as *mut c_void, env_ptrs, 0, 2);
        let thunk_mixed = dx_rt_closure_create(mixed_ptr_thunk as *mut c_void, env_mixed, 0, 3);

        assert_eq!(dx_rt_thunk_call_ptr(thunk_ptrs), payload1);
        assert_eq!(dx_rt_thunk_call_ptr(thunk_mixed), payload1);

        free_closure(thunk_ptrs);
        free_closure(thunk_mixed);
        free_env::<Capture2Ptr>(env_ptrs);
        free_env::<CaptureI64PtrI1>(env_mixed);
    }

    #[test]
    fn thunk_ptr_calls_can_dispatch_with_three_ptr_captures() {
        let payload0 = 0x1234usize as *mut c_void;
        let payload1 = 0x5678usize as *mut c_void;
        let payload2 = 0x9abcusize as *mut c_void;
        let env = Box::into_raw(Box::new(Capture3Ptr {
            a: payload0,
            b: payload1,
            c: payload2,
        })) as EnvHandle;
        let thunk = dx_rt_closure_create(third_ptr_thunk as *mut c_void, env, 0, 3);

        assert_eq!(dx_rt_thunk_call_ptr(thunk), payload2);

        free_closure(thunk);
        free_env::<Capture3Ptr>(env);
    }

    #[test]
    fn thunk_i64_calls_can_dispatch_with_three_captures() {
        let env = Box::into_raw(Box::new(Capture3I64 {
            a: 10,
            b: 11,
            c: 21,
        })) as EnvHandle;
        let thunk = dx_rt_closure_create(sum_three_i64_captures as *mut c_void, env, 0, 3);

        assert_eq!(dx_rt_thunk_call_i64(thunk), 42);

        free_closure(thunk);
        free_env::<Capture3I64>(env);
    }

    #[test]
    fn thunk_f64_and_i1_calls_can_dispatch_with_three_captures() {
        let env_f64 = Box::into_raw(Box::new(Capture3F64 {
            a: 10.0,
            b: 11.0,
            c: 21.0,
        })) as EnvHandle;
        let env_i1 = Box::into_raw(Box::new(Capture3I1 {
            a: true,
            b: false,
            c: false,
        })) as EnvHandle;
        let thunk_f64 =
            dx_rt_closure_create(sum_three_f64_captures as *mut c_void, env_f64, 0, 3);
        let thunk_i1 =
            dx_rt_closure_create(xor_three_i1_thunk_captures as *mut c_void, env_i1, 0, 3);

        assert_eq!(dx_rt_thunk_call_f64(thunk_f64), 42.0);
        assert!(dx_rt_thunk_call_i1(thunk_i1));

        free_closure(thunk_f64);
        free_closure(thunk_i1);
        free_env::<Capture3F64>(env_f64);
        free_env::<Capture3I1>(env_i1);
    }

    #[test]
    fn thunk_void_calls_can_dispatch_with_zero_or_mixed_three_captures() {
        let payload = 0x1234usize as *mut c_void;
        let env = Box::into_raw(Box::new(CaptureI64PtrI1 {
            i: 99,
            p: payload,
            b: true,
        })) as EnvHandle;
        let thunk0 = dx_rt_closure_create(mark_void_thunk as *mut c_void, ptr::null_mut(), 0, 0);
        let thunk_mixed = dx_rt_closure_create(mixed_void_thunk as *mut c_void, env, 0, 3);

        VOID_THUNK_CALLED.store(false, Ordering::SeqCst);
        dx_rt_thunk_call_void(thunk0);
        assert!(VOID_THUNK_CALLED.load(Ordering::SeqCst));

        VOID_THUNK_CALLED.store(false, Ordering::SeqCst);
        dx_rt_thunk_call_void(thunk_mixed);
        assert!(VOID_THUNK_CALLED.load(Ordering::SeqCst));

        free_closure(thunk0);
        free_closure(thunk_mixed);
        free_env::<CaptureI64PtrI1>(env);
    }

    #[test]
    fn ordinary_closure_void_call_can_dispatch_with_mixed_three_capture_env() {
        let payload = 0x1234usize as *mut c_void;
        let env = Box::into_raw(Box::new(CaptureI64PtrI1 {
            i: 99,
            p: payload,
            b: true,
        })) as EnvHandle;
        let closure = dx_rt_closure_create(consume_with_mixed_capture as *mut c_void, env, 3, 3);

        dx_rt_closure_call_void_3_i64_ptr_i1(closure, 1, payload, true);

        free_closure(closure);
        free_env::<CaptureI64PtrI1>(env);
    }

    #[test]
    fn match_tag_compares_nominal_tag_strings() {
        let ok = CString::new("Ok").expect("cstring");
        let err = CString::new("Err").expect("cstring");
        let value = tagged_value(&ok);

        assert!(dx_rt_match_tag(value, ok.as_ptr()));
        assert!(!dx_rt_match_tag(value, err.as_ptr()));

        free_tagged_value(value);
    }

    #[test]
    fn python_hooks_stub_to_null() {
        let name = CString::new("read_csv").expect("cstring");
        let method = CString::new("head").expect("cstring");
        assert!(dx_rt_py_call_function(name.as_ptr(), 1).is_null());
        assert!(dx_rt_py_call_method(ptr::null_mut(), method.as_ptr(), 0).is_null());
        assert!(dx_rt_py_call_dynamic(ptr::null_mut(), 0).is_null());
    }
}
