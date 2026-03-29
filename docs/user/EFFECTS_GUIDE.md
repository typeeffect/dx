# Effects Guide

*This guide is in preparation. Effects and handlers are the core differentiating
feature of DX — this document will be written carefully, not rushed.*

For now, see:

- Target examples: `examples/targets/effects_run.dx.example`, `effects_compose.dx.example`
- AD via effects: `examples/targets/ad_scalar_grad.dx.example`

## What works today

- Effect annotations on functions (`!io`, `!py`, `!throw`) parse and are tracked
- Effectful `main` is rejected by the executable contract
- The smooth and prob effects are demonstrated in target examples

## What is preview syntax

- `handle...with` blocks
- Handler definitions
- `resume` / continuation calls
- Multi-shot (`multi`) handlers
- `smooth_primitive` and `backward` blocks
