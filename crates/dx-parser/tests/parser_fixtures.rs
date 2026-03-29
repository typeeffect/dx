//! Integration tests that drive parser fixtures.
//!
//! Fixture files live in tests/fixtures/:
//!   - should_parse.dx    — snippets that must parse today
//!   - future_unsupported.dx — approved syntax not yet parseable (expected to fail)
//!
//! Format: sections separated by "--- name" lines. Lines starting with "##" are comments.

use dx_parser::{Lexer, Parser};

/// Split a fixture file into named sections.
fn load_sections(text: &str) -> Vec<(&str, String)> {
    let mut sections = Vec::new();
    let mut current_name: Option<&str> = None;
    let mut current_body = String::new();

    for line in text.lines() {
        if line.starts_with("##") {
            continue;
        }
        if let Some(name) = line.strip_prefix("--- ") {
            if let Some(prev_name) = current_name {
                let body = current_body.trim().to_string();
                if !body.is_empty() {
                    sections.push((prev_name, body));
                }
            }
            current_name = Some(name.trim());
            current_body = String::new();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if let Some(name) = current_name {
        let body = current_body.trim().to_string();
        if !body.is_empty() {
            sections.push((name, body));
        }
    }
    sections
}

fn try_parse(src: &str) -> Result<(), String> {
    let tokens = Lexer::new(src).tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_module().map(|_| ()).map_err(|e| e.message)
}

// ── should-parse fixtures ────────────────────────────────────────

#[test]
fn all_should_parse_fixtures_succeed() {
    let text = include_str!("fixtures/should_parse.dx");
    let sections = load_sections(text);
    assert!(
        !sections.is_empty(),
        "no sections found in should_parse.dx"
    );

    let mut failures = Vec::new();
    for (name, body) in &sections {
        if let Err(msg) = try_parse(body) {
            failures.push(format!("  [{name}]: {msg}"));
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} fixture(s) failed to parse:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

/// Run each should-parse fixture individually so failures are easy to locate.
macro_rules! should_parse_test {
    ($name:ident, $fixture_name:expr) => {
        #[test]
        fn $name() {
            let text = include_str!("fixtures/should_parse.dx");
            let sections = load_sections(text);
            let (_, body) = sections
                .iter()
                .find(|(n, _)| *n == $fixture_name)
                .unwrap_or_else(|| panic!("fixture '{}' not found", $fixture_name));
            if let Err(msg) = try_parse(body) {
                panic!("fixture '{}' failed: {}", $fixture_name, msg);
            }
        }
    };
}

should_parse_test!(fixture_nested_member_chains, "nested_member_chains");
should_parse_test!(fixture_long_member_chain, "long_member_chain");
should_parse_test!(fixture_python_boundary, "python_boundary");
should_parse_test!(fixture_python_numpy_wrapper, "python_numpy_wrapper");
should_parse_test!(fixture_multi_import_names, "multi_import_names");
should_parse_test!(fixture_lazy_expression, "lazy_expression");
should_parse_test!(fixture_lazy_block, "lazy_block");
should_parse_test!(fixture_lazy_param_type_plain, "lazy_param_type_plain");
should_parse_test!(fixture_lazy_param_type_with_effects, "lazy_param_type_with_effects");
should_parse_test!(fixture_lazy_param_type_multi_effect, "lazy_param_type_multi_effect");
should_parse_test!(fixture_named_arguments, "named_arguments");
should_parse_test!(fixture_mixed_positional_and_named, "mixed_positional_and_named");
should_parse_test!(fixture_single_param_lambda, "single_param_lambda");
should_parse_test!(fixture_multi_param_lambda, "multi_param_lambda");
should_parse_test!(fixture_typed_lambda, "typed_lambda");
should_parse_test!(fixture_block_lambda, "block_lambda");
should_parse_test!(fixture_if_else, "if_else");
should_parse_test!(fixture_if_elif_else, "if_elif_else");
should_parse_test!(fixture_nested_if, "nested_if");
should_parse_test!(fixture_if_as_expression, "if_as_expression");
should_parse_test!(fixture_match_simple, "match_simple");
should_parse_test!(fixture_match_constructor_patterns, "match_constructor_patterns");
should_parse_test!(fixture_match_as_expression, "match_as_expression");
should_parse_test!(fixture_val_var_rebind, "val_var_rebind");
should_parse_test!(fixture_placeholder_member, "placeholder_member");
should_parse_test!(fixture_placeholder_in_call, "placeholder_in_call");
should_parse_test!(fixture_me_member_access, "me_member_access");
should_parse_test!(fixture_it_pipeline, "it_pipeline");
should_parse_test!(fixture_function_type_return, "function_type_return");
should_parse_test!(fixture_zero_arg_function_type_return, "zero_arg_function_type_return");
should_parse_test!(fixture_multiple_effects, "multiple_effects");
should_parse_test!(fixture_no_return_type, "no_return_type");
should_parse_test!(fixture_multiple_functions, "multiple_functions");
should_parse_test!(fixture_empty_function, "empty_function");
should_parse_test!(fixture_nested_calls, "nested_calls");
should_parse_test!(fixture_string_and_int_literals, "string_and_int_literals");
should_parse_test!(fixture_lazy_in_call_position, "lazy_in_call_position");

// examples from DX_LONG_EXAMPLES.md (operator-free fragments)
should_parse_test!(fixture_ex10_cache_pattern, "ex10_cache_pattern");
should_parse_test!(fixture_ex23_deferred_fallback, "ex23_deferred_fallback_simplified");
should_parse_test!(fixture_ex3_lazy_thunk_param_if, "ex3_lazy_thunk_param_if");
should_parse_test!(fixture_ex10_lazy_block_in_call, "ex10_lazy_block_in_call");

// composite patterns
should_parse_test!(fixture_match_multi_statement_arms, "match_multi_statement_arms");
should_parse_test!(fixture_match_three_arms, "match_three_arms");
should_parse_test!(fixture_match_inside_if, "match_inside_if");
should_parse_test!(fixture_if_inside_match, "if_inside_match");
should_parse_test!(fixture_it_chain_multi_step, "it_chain_multi_step");
should_parse_test!(fixture_deep_nested_calls, "deep_nested_calls");
should_parse_test!(fixture_lambda_returning_member_chain, "lambda_returning_member_chain");
should_parse_test!(fixture_multi_val_then_expr, "multi_val_then_expr");
should_parse_test!(fixture_call_with_multiple_lazy_args, "call_with_multiple_lazy_args");
should_parse_test!(fixture_match_followed_by_stmts, "match_followed_by_stmts");
should_parse_test!(fixture_if_else_both_multi_stmt, "if_else_both_multi_stmt");
should_parse_test!(fixture_member_call_chain_long, "member_call_chain_long");
should_parse_test!(fixture_lazy_param_py_effect, "lazy_param_py_effect");

// regression: named + lazy mixed calls, multi-import, nesting combos
should_parse_test!(fixture_named_and_lazy_mixed_call, "named_and_lazy_mixed_call");
should_parse_test!(fixture_named_args_with_lazy_value, "named_args_with_lazy_value");
should_parse_test!(fixture_multiple_py_imports, "multiple_py_imports");
should_parse_test!(fixture_match_if_in_every_arm, "match_if_in_every_arm");
should_parse_test!(fixture_match_with_member_access_scrutinee, "match_with_member_access_scrutinee");
should_parse_test!(fixture_if_with_match_in_both_branches, "if_with_match_in_both_branches");
should_parse_test!(fixture_lazy_param_alongside_normal, "lazy_param_alongside_normal_params");

// operator expressions (newly parseable)
should_parse_test!(fixture_binary_operators, "binary_operators");
should_parse_test!(fixture_string_concatenation, "string_concatenation");
should_parse_test!(fixture_operator_in_lambda_body, "operator_in_lambda_body");
should_parse_test!(fixture_nested_conditionals_with_operators, "nested_conditionals_with_operators");
should_parse_test!(fixture_placeholder_with_operators, "placeholder_with_operators");
should_parse_test!(fixture_comparison_on_member_access, "comparison_on_member_access");
should_parse_test!(fixture_operators_in_full_example, "operators_in_full_example");
should_parse_test!(fixture_unit_literal, "unit_literal");
should_parse_test!(fixture_gt_and_gte_operators, "gt_and_gte_operators");
should_parse_test!(fixture_schema_decl_basic, "schema_decl_basic");
should_parse_test!(fixture_schema_decl_using_refresh, "schema_decl_using_refresh");

// ── future-unsupported fixtures ──────────────────────────────────

#[test]
fn all_future_unsupported_fixtures_currently_fail() {
    let text = include_str!("fixtures/future_unsupported.dx");
    let sections = load_sections(text);
    assert!(
        !sections.is_empty(),
        "no sections found in future_unsupported.dx"
    );

    let mut unexpected_successes = Vec::new();
    for (name, body) in &sections {
        if try_parse(body).is_ok() {
            unexpected_successes.push(*name);
        }
    }
    if !unexpected_successes.is_empty() {
        panic!(
            "These future fixtures unexpectedly parsed successfully (promote them to should_parse.dx):\n  {}",
            unexpected_successes.join(", ")
        );
    }
}
