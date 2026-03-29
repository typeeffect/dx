# DX v0-alpha.1 Release Checklist

## Release Narrative

`dx v0-alpha.1` is the first tagged release of the bootstrap compiler.
It ships four concrete slices:

1. **Native deterministic core** — a real compiler pipeline producing real
   native executables with a proven runnable subset
2. **Schema tooling slice** — artifact parser, validator, and CLI tools for
   compile-time schema acquisition
3. **Memory runtime slice** — arena, shared buffer, tensor storage, pool,
   and FFI boundary types
4. **Target-demo package** — 18 semantic target examples recovered from dx-03
   covering effects, AD, PPL, multi-shot, and ML

## Already Ready

- [ ] `cargo test --workspace` green (900+ tests)
- [ ] 12 executable-entry fixtures proven end-to-end
- [ ] Manifest-driven execution proof passes
- [ ] `dx-build-exec` and `dx-run-exec` black-box tested
- [ ] `dx-schema-validate`, `dx-schema-new`, `dx-schema-match` CLIs working
- [ ] `.dxschema` v0.1.0 artifact format spec + canonical validation
- [ ] `dx-memory` crate: arena, buffer, pool, tensor, FFI boundary tested
- [ ] 18 target examples across 6 tranches
- [ ] Foundations paper trail doc linking papers → claims → demos

## Must Do Before Tagging

- [ ] Commit all uncommitted work (many files from tasks 38-87 are untracked/modified)
- [ ] Verify `cargo test --workspace` green on a clean checkout
- [ ] Remove or archive stale `CLAUDE_DONE/` / `CLAUDE_REPORT/` task files
- [ ] Verify `scripts/prove_executable_entry_subset.sh` passes from clean state
- [ ] Add `version = "0.0.1-alpha.1"` to workspace `Cargo.toml` if not already set
- [ ] Tag: `git tag v0-alpha.1`

## Should Do (Recommended)

- [ ] Update `README.md` to reference `docs/DX_V0_ALPHA_RELEASE_SUMMARY.md`
- [ ] Add a one-paragraph CHANGELOG entry
- [ ] Verify no secrets or credentials in tracked files
- [ ] Run `cargo clippy --workspace` and address any warnings
- [ ] Verify the `examples/schema/` canonical artifacts are up to date

## Explicitly Out of Scope

These are real work items but not gates for `v0-alpha.1`:

- Effect/handler syntax in the parser
- `schema` keyword or compiler integration
- Arena/region/tensor DX language syntax
- AD or probabilistic programming implementation
- Float literals in the parser
- `main` with arguments, `Unit` return, or effects
- CI/CD pipeline
- `cargo publish` readiness
- Documentation website or rendered docs

## Release Verification

After tagging, verify:

```bash
git stash && cargo test --workspace && git stash pop
scripts/prove_executable_entry_subset.sh
cargo run -p dx-schema --bin dx-schema-validate -- examples/schema/customers.dxschema.example
cargo test -p dx-memory
```
