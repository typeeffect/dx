pub mod archive;
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
    env: EnvHandle,
    arity: u32,
}

#[repr(C)]
struct StubTaggedValue {
    tag: Utf8Ptr,
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_closure_create(env: EnvHandle, arity: u32) -> ClosureHandle {
    let closure = Box::new(StubClosure { env, arity });
    Box::into_raw(closure) as ClosureHandle
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_i64(_closure: ClosureHandle) -> i64 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_f64(_closure: ClosureHandle) -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_i1(_closure: ClosureHandle) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn dx_rt_thunk_call_ptr(_closure: ClosureHandle) -> *mut c_void {
    ptr::null_mut()
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
        let handle = dx_rt_closure_create(ptr::null_mut(), 0);
        assert!(!handle.is_null());
        free_closure(handle);
    }

    #[test]
    fn thunk_calls_return_default_values() {
        let handle = dx_rt_closure_create(ptr::null_mut(), 0);
        assert_eq!(dx_rt_thunk_call_i64(handle), 0);
        assert_eq!(dx_rt_thunk_call_f64(handle), 0.0);
        assert!(!dx_rt_thunk_call_i1(handle));
        assert!(dx_rt_thunk_call_ptr(handle).is_null());
        dx_rt_thunk_call_void(handle);
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
