# DX Provider Constraints

## Purpose

This document defines the hard limits on compile-time providers in `dx`.

The goal is not to weaken providers.
The goal is to prevent them from becoming:

- a macro system in disguise
- a plugin backdoor
- a source of dialects
- a way to bypass the small-core rule

## Core Rule

Providers may extend **compile-time semantics** in predefined slots.

Providers may not define new **general language syntax**, new **core semantic
rules**, or new **dialects** of `dx`.

Short version:

- extensible semantics
- not extensible grammar
- not extensible language identity

## No-Dialect Rule

Providers must not be a path to Babel.

That means providers may not:

- invent new block forms
- introduce new statement families
- alter parsing rules
- redefine existing keywords
- create local sublanguages that ordinary DX tooling cannot understand

All provider declarations must live inside syntax already reserved by the
language.

## Allowed Powers

Providers may:

- consume explicit source declarations
- validate narrow provider-specific contracts
- read or produce locked artifacts
- contribute typed metadata to the compiler
- expose compiler-visible results inside predefined semantic slots

Examples:

- schema declaration introducing a future row type
- AD primitive declaration introducing primitive metadata

## Forbidden Powers

Providers must not:

- rewrite arbitrary program ASTs in v0
- add new top-level grammar categories freely
- execute arbitrary compile-time I/O during normal builds
- perform hidden network access
- redefine the type system
- redefine the effect system
- create provider-local syntax that looks unlike DX
- depend on host-language-only extension points invisible to DX

## Build Discipline

Normal build should remain:

- deterministic
- offline-capable where possible
- artifact-consuming, not artifact-discovering

So providers must follow these rules:

- refresh/generation is explicit
- build consumes locked artifacts
- missing or mismatched artifacts fail clearly
- no implicit discovery during ordinary compilation

## Slot-Based Extensibility

Providers should only fill slots the core already recognizes.

Good slot examples:

- schema declarations
- future AD primitive declarations

Bad examples:

- "provider-defined syntax blocks"
- "custom grammar loaded from a package"
- "provider decides what new keywords mean"

This is how `dx` stays one language instead of a federation of local variants.

## Why This Matters

If providers are too powerful, `dx` loses:

- readability
- toolability
- LLM-friendliness
- reviewability
- language identity

The project would gain "extensibility" and lose coherence.

That tradeoff is wrong for `dx`.

## Relation To The Small Core Rule

Providers exist to keep the core small.

But if they are unconstrained, they simply move core inflation to another
location.

So the two rules must be held together:

- core stays small
- providers stay narrow

Reference:

- `docs/DX_SMALL_CORE_RULES.md`

## Relation To Provider Kinds

### Schema Providers

Allowed:

- source declaration
- locked artifact
- typed metadata for future row integration

Not allowed:

- ad hoc datasource mini-languages
- arbitrary query DSLs injected into the grammar

### AD Primitive Providers

Allowed:

- forward/backward contract
- generated or explicit derivative metadata
- future primitive artifacts

Not allowed:

- unrestricted symbolic rewrite systems
- arbitrary compile-time compiler plugins

## Evaluation Rule For New Provider Ideas

When adding a new provider kind, ask:

1. Does it fill a slot the language already reserves?
2. Is the declaration still ordinary DX surface?
3. Is the result typed and compiler-visible?
4. Is the build behavior deterministic?
5. Does it avoid creating a local dialect?

If any answer is "no", the provider design is probably wrong.

## v0 Guidance

In v0, providers should be stricter than they may be later.

That means:

- declaration-first
- artifact-first where relevant
- no general AST rewrites
- no host-only plugin escape hatch
- no provider-defined syntax families

This preserves space to relax carefully later without starting from chaos.

## Strategic Rule

Providers are for controlled capability growth, not for language fragmentation.

If a provider proposal makes DX programs stop looking like DX, the proposal
should be rejected.

## Related Docs

- `docs/DX_SMALL_CORE_RULES.md`
- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`
- `docs/DX_AD_PRIMITIVE_PROVIDER_PLAN.md`
