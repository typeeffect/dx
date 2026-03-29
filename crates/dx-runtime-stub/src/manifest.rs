pub const EXPORTED_SYMBOLS: &[&str] = &[
    "dx_rt_closure_create",
    "dx_rt_closure_call_i64_1_i64",
    "dx_rt_closure_call_i64_2_i64_i64",
    "dx_rt_closure_call_ptr_1_ptr",
    "dx_rt_closure_call_ptr_1_i64",
    "dx_rt_closure_call_ptr_2_ptr_i64",
    "dx_rt_closure_call_void_3_i64_ptr_i1",
    "dx_rt_thunk_call_i64",
    "dx_rt_thunk_call_f64",
    "dx_rt_thunk_call_i1",
    "dx_rt_thunk_call_ptr",
    "dx_rt_thunk_call_void",
    "dx_rt_match_tag",
    "dx_rt_throw_check_pending",
    "dx_rt_py_call_function",
    "dx_rt_py_call_method",
    "dx_rt_py_call_dynamic",
];

pub fn render_exported_symbols() -> String {
    EXPORTED_SYMBOLS.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_symbols_include_core_stub_surface() {
        for symbol in &[
            "dx_rt_closure_create",
            "dx_rt_closure_call_i64_1_i64",
            "dx_rt_thunk_call_i64",
            "dx_rt_match_tag",
            "dx_rt_throw_check_pending",
            "dx_rt_py_call_function",
        ] {
            assert!(EXPORTED_SYMBOLS.contains(symbol), "missing {symbol}");
        }
    }

    #[test]
    fn exported_symbols_claim_ordinary_closure_call_surface() {
        assert!(EXPORTED_SYMBOLS
            .iter()
            .any(|sym| sym.starts_with("dx_rt_closure_call_")));
    }

    #[test]
    fn rendered_symbols_are_deterministic() {
        assert_eq!(render_exported_symbols(), render_exported_symbols());
    }
}
