# DX Foundations Paper Trail

## Purpose

This document maps the academic papers that ground `dx`'s core design
to the specific language claims they support and the target examples
that demonstrate those claims.

It is not a literature survey.
It is a navigable trail from papers to design decisions to working demos.

## Core Papers

### Algebraic Effects and Handlers

**Pretnar 2015** — *An Introduction to Algebraic Effects and Handlers*
`paper/pretnar-2015-introduction-to-algebraic-effects-and-handlers.pdf`

- **dx claim**: effects are first-class; handlers define control semantics
- **target examples**: `effects_run`, `effects_compose`, `nested_fun_handle`

**Plotkin & Pretnar 2013** — *Handling Algebraic Effects*
`paper/plotkin-pretnar-2013-handling-algebraic-effects.pdf`

- **dx claim**: handlers are a uniform mechanism for exceptions, state, I/O, and non-determinism
- **target examples**: `effects_run` (never/once handlers), `amb_basic` (multi handlers)

### Selective CPS and No-Coloring

**Leijen 2016** — *Type Directed Compilation of Row-typed Algebraic Effects*
`paper/leijen-2016-type-directed-compilation-of-row-typed-algebraic-effects.pdf`

- **dx claim**: compile-time effect information eliminates function coloring;
  only multi-shot handlers need CPS translation; once/never compile to direct style
- **target examples**: `async_no_coloring`, `amb_queens` (selective CPS for multi-shot)
- **internal draft**: `SELECTIVE_CPS_PAPER.md` — specializes Leijen's approach
  to `dx` with explicit `never`/`once`/`multi` multiplicities

**Lindley, McBride, McLaughlin 2017** — *Do Be Do Be Do*
`paper/lindley-mcbride-mclaughlin-2017-do-be-do-be-do.pdf`

- **dx claim**: structured effectful control can be readable and composable
- **target examples**: `effects_compose` (handler nesting order)

### Practical Effect Systems

**Sivaramakrishnan et al. 2021** — *Retrofitting Effect Handlers onto OCaml*
`paper/sivaramakrishnan-et-al-2021-retrofitting-effect-handlers-onto-ocaml.pdf`

- **dx claim**: effect handlers can be retrofitted into a real runtime
  without destroying performance
- **relevance**: informs the `dx` runtime strategy (GraalVM/Truffle in dx-03,
  LLVM native in dx-04)

### AD via Effects

**Sigal 2024** — *Automatic Differentiation via Effects and Handlers*
`paper/sigal-automatic-differentiation-via-effects-and-handlers.pdf`

- **dx claim**: AD is an effect, not a compiler builtin; `grad()` is a
  user-level function written with `handle...with` over the `smooth` effect
- **target examples**: `ad_scalar_grad`, `ad_custom_grad`, `ad_full_smooth_handler`
- **thesis**: `jsigal.com/assets/pdf/thesis.pdf` — Koka implementation
  that dx-03 proved end-to-end on GraalVM

## Paper → dx Claim → Target Example Map

| Paper | dx Claim | Target Examples |
|-------|----------|-----------------|
| Pretnar 2015 | Effects and handlers are first-class | `effects_run`, `effects_compose`, `nested_fun_handle` |
| Plotkin & Pretnar 2013 | Handlers unify exception, state, I/O | `effects_run`, `amb_basic` |
| Leijen 2016 | Selective CPS eliminates coloring | `async_no_coloring`, `amb_queens` |
| Lindley et al. 2017 | Structured effectful control | `effects_compose` |
| Sivaramakrishnan et al. 2021 | Practical runtime for handlers | (runtime architecture) |
| Sigal 2024 | AD as an effect | `ad_scalar_grad`, `ad_custom_grad`, `ad_full_smooth_handler` |

## Derived dx Design Decisions

### "grad() is a dx function"

Papers: Sigal 2024 + Leijen 2016

The smooth effect intercepts arithmetic. The handler builds a tape.
`grad()` is written in dx, not implemented in the compiler. This means
custom gradient behavior (tracing, clipping, scaling) is just a different
handler — no `custom_vjp` decorator needed.

Demonstrated: `ad_custom_grad`, `ad_full_smooth_handler`

### "Probabilistic models are effectful functions"

Papers: Pretnar 2015 + Plotkin & Pretnar 2013

The `prob` effect provides `sample` and `observe`. Different handlers
implement different inference strategies (importance sampling, MCMC, HMC).
The model function never changes.

Demonstrated: `prob_basic_inference`, `prob_bayesian_regression`, `prob_hmc_regression`

### "Three effects compose in one program"

Papers: Leijen 2016 + Sigal 2024

HMC requires AD (smooth), randomness (random), and mutable state — three
effects interacting. Effect handlers compose naturally; no special
integration needed.

Demonstrated: `prob_hmc_regression`

### "No async/await coloring"

Papers: Leijen 2016 (Section 6: async/await as effects)

I/O is a `once` effect. It compiles to direct-style dispatch with zero
overhead. No function coloring, no viral `async` propagation.

Demonstrated: `async_no_coloring`

### "Multi-shot is real, not theoretical"

Papers: Leijen 2016 + Plotkin & Pretnar 2013

`multi` effects (like `amb`) use multi-shot continuations. The compiler
applies CPS only to `handle` blocks that need it. Everything else stays
direct-style.

Demonstrated: `amb_basic`, `amb_collect`, `amb_queens`

## What Is Not Covered By Papers

The following dx design areas are not directly grounded in the papers above:

- genitivo sassone field access (`'` syntax)
- compile-time schema providers
- the three-layer memory model (arena / shared buffer / tensor)
- LLVM native compilation strategy (dx-04 direction)
- Python interop via `from py ... import ...`

These are original dx design decisions informed by practical needs, not
academic foundations.

## References

- Paper folder: `/home/frapas/dx-lang/dx-04/paper/`
- Internal draft: `/home/frapas/dx-lang/dx-04/SELECTIVE_CPS_PAPER.md`
- Target examples: `examples/targets/`
- Recovery tracker: `docs/DX_TARGET_EXAMPLES_RECOVERY.md`
- AD/PPL direction: `docs/DX_AD_PPL_DIRECTION.md`
