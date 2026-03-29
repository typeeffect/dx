Work in the repository root.

Read these files first:
- crates/dx-llvm-ir/src/bin/dx-build-exec.rs
- crates/dx-llvm-ir/src/exec.rs
- docs/DX_EXECUTION_WORKFLOW.md
- docs/DX_TOOLCHAIN_PROVEN_SUBSET.md
- scripts/build_backend_demo.sh
- crates/dx-llvm-ir/tests/cli_workflow.rs
- crates/dx-llvm-ir/tests/demo_fixtures.rs

Important current status:
- the bash workflow surface is now heavily source-of-truth-driven
- `dx-build-exec` now exists as a native CLI for the `.dx -> .ll -> .bc -> .o -> exe` path
- `dx-build-exec` already supports:
  - `--verify`
  - `--dry-run`
  - `--runtime-archive <path>`
- do not redesign backend semantics
- do not touch parser / HIR / MIR / runtime lowering
- keep this pass black-box and operational

Task:
Harden the native executable-build CLI surface (`dx-build-exec`) with fixture-based tests and workflow documentation.

This is not a semantics pass.
It is a native CLI/workflow pass.

You may edit:
- crates/dx-llvm-ir/tests/*
- crates/dx-llvm-ir/src/bin/*
- docs/DX_EXECUTION_WORKFLOW.md
- docs/DX_TOOLCHAIN_PROVEN_SUBSET.md
- Makefile
- scripts/*
- CLAUDE_TODO/*

Do not edit:
- crates/dx-parser/*
- crates/dx-hir/*
- crates/dx-mir/*
- crates/dx-runtime/src/*
- crates/dx-codegen/src/*
- crates/dx-llvm/src/*
- runtime ABI docs unless wording cleanup is strictly necessary

Primary goals:

1. Add black-box coverage for `dx-build-exec --dry-run` on canonical demos.
2. Verify the rendered native build plan stays coherent with the current executable subset.
3. Sync docs / workflow entrypoints so the native CLI is clearly part of the supported backend flow.

Good targets:
- fixture-based tests for `dx-build-exec --dry-run`
- assertions on:
  - `dx-emit-llvm`
  - `llvm-as`
  - `llc`
  - `cc`
  - runtime archive path handling
- at least one test with `--runtime-archive <path>`
- docs updates if they materially improve the current workflow
- optional Makefile target(s) only if clearly useful

Non-goals:
- no lowering changes
- no validator changes
- no runtime semantic work
- no LLVM installation work
- no new demo semantics

Acceptance criteria:
- `cargo test -q` passes
- `dx-build-exec --dry-run` is covered for canonical fixtures
- docs remain truthful about the native executable-build path
- summarize exactly what changed
- list touched files
- explicitly note the next real blind spots after this pass
