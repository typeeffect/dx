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
}

#[repr(C)]
struct StubTaggedValue {
    tag: Utf8Ptr,
}

fn closure_env_ptr<T>(closure: ClosureHandle) -> Option<*const T> {
    if closure.is_null() {
        return None;
    }
    let closure = unsafe { &*(closure as *const StubClosure) };
    if closure.env.is_null() {
        return None;
    }
    Some(closure.env as *const T)
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_create(
    code_ptr: *mut c_void,
    env: EnvHandle,
    arity: u32,
) -> ClosureHandle {
    let closure = Box::new(StubClosure { code_ptr, env, arity });
    Box::into_raw(closure) as ClosureHandle
}

macro_rules! closure_call_stub {
    ($name:ident($($arg:ident : $ty:ty),*) -> $ret:ty => $default:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(_closure: ClosureHandle, $($arg: $ty),*) -> $ret {
            let _ = ($($arg),*);
            $default
        }
    };
}

macro_rules! closure_call_stub_void {
    ($name:ident($($arg:ident : $ty:ty),*)) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(_closure: ClosureHandle, $($arg: $ty),*) {
            let _ = ($($arg),*);
        }
    };
}

closure_call_stub!(dx_rt_closure_call_i64_1_i64(arg0: i64) -> i64 => 0);
closure_call_stub!(dx_rt_closure_call_i64_2_i64_i64(arg0: i64, arg1: i64) -> i64 => 0);
closure_call_stub!(dx_rt_closure_call_ptr_1_ptr(arg0: *mut c_void) -> *mut c_void => ptr::null_mut());
closure_call_stub!(dx_rt_closure_call_ptr_1_i64(arg0: i64) -> *mut c_void => ptr::null_mut());
closure_call_stub!(dx_rt_closure_call_ptr_2_ptr_i64(arg0: *mut c_void, arg1: i64) -> *mut c_void => ptr::null_mut());
closure_call_stub_void!(dx_rt_closure_call_void_3_i64_ptr_i1(arg0: i64, arg1: *mut c_void, arg2: bool));

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_i64(closure: ClosureHandle) -> i64 {
    closure_env_ptr::<i64>(closure)
        .map(|ptr| unsafe { *ptr })
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_f64(closure: ClosureHandle) -> f64 {
    closure_env_ptr::<f64>(closure)
        .map(|ptr| unsafe { *ptr })
        .unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_i1(closure: ClosureHandle) -> bool {
    closure_env_ptr::<bool>(closure)
        .map(|ptr| unsafe { *ptr })
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_ptr(closure: ClosureHandle) -> *mut c_void {
    closure_env_ptr::<*mut c_void>(closure)
        .map(|ptr| unsafe { *ptr })
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_void(_closure: ClosureHandle) {}

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
        let handle = dx_rt_closure_create(ptr::null_mut(), ptr::null_mut(), 0);
        assert!(!handle.is_null());
        free_closure(handle);
    }

    #[test]
    fn thunk_calls_return_default_values() {
        let handle = dx_rt_closure_create(ptr::null_mut(), ptr::null_mut(), 0);
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

        let thunk_i64 = dx_rt_closure_create(ptr::null_mut(), env_i64, 0);
        let thunk_f64 = dx_rt_closure_create(ptr::null_mut(), env_f64, 0);
        let thunk_i1 = dx_rt_closure_create(ptr::null_mut(), env_i1, 0);
        let thunk_ptr = dx_rt_closure_create(ptr::null_mut(), env_ptr, 0);

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
        let handle = dx_rt_closure_create(ptr::null_mut(), ptr::null_mut(), 1);
        assert_eq!(dx_rt_closure_call_i64_1_i64(handle, 7), 0);
        assert_eq!(dx_rt_closure_call_i64_2_i64_i64(handle, 7, 9), 0);
        assert!(dx_rt_closure_call_ptr_1_ptr(handle, ptr::null_mut()).is_null());
        assert!(dx_rt_closure_call_ptr_1_i64(handle, 7).is_null());
        assert!(dx_rt_closure_call_ptr_2_ptr_i64(handle, ptr::null_mut(), 9).is_null());
        dx_rt_closure_call_void_3_i64_ptr_i1(handle, 1, ptr::null_mut(), false);
        free_closure(handle);
    }

    #[test]
    fn closure_create_preserves_code_pointer() {
        let sentinel = 0x1234usize as *mut c_void;
        let handle = dx_rt_closure_create(sentinel, ptr::null_mut(), 1);
        let closure = unsafe { &*(handle as *const StubClosure) };
        assert_eq!(closure.code_ptr, sentinel);
        free_closure(handle);
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
