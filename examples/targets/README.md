# DX Target Examples

This package collects semantic target examples recovered from `dx-03`.

These files are not parser-committed fixtures.
They are design targets written in the newer DX style so the repo keeps the strongest language demos visible while the exact effect and handler surface is still settling.

## Why These Files Exist

- preserve the best `dx-03` demos without transplanting old syntax blindly
- keep async decoloring, effect composition, nested local functions, AD, and probabilistic programming visible as explicit targets
- give the repo a stable place for "this is what DX should still be able to express"

## Provisional Status

- files use `.dx.example`
- function syntax follows the newer block-based DX direction
- effect declaration and handler syntax are still provisional
- semantics matter more than exact tokens

## Current Tranches

### Effects and Async Decoloring

- [effects_run.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/effects_run.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/effects_run.dx`
- [effects_compose.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/effects_compose.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/effects_compose.dx`
- [async_no_coloring.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/async_no_coloring.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/async_no_coloring.dx`
- [nested_fun_handle.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/nested_fun_handle.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/nested_fun_handle.dx`

### AD and Probabilistic Programming

- [ad_scalar_grad.dx.example](ad_scalar_grad.dx.example)
  Source: `dx-03/src/test/resources/programs/ad.dx`
- [ad_custom_grad.dx.example](ad_custom_grad.dx.example)
  Source: `dx-03/src/test/resources/programs/dx_custom_grad.dx`
- [prob_basic_inference.dx.example](prob_basic_inference.dx.example)
  Source: `dx-03/src/test/resources/programs/prob_basic.dx`
- [prob_monty_hall.dx.example](prob_monty_hall.dx.example)
  Source: `dx-03/src/test/resources/programs/prob_monty.dx`

### Multi-shot Search and Selective CPS

- [amb_basic.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/amb_basic.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/amb_basic.dx`
- [amb_collect.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/amb_collect.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/amb_collect.dx`
- [amb_queens.dx.example](/home/frapas/dx-lang/dx-04/dx-bootstrap/examples/targets/amb_queens.dx.example)
  Source: `/home/frapas/dx-lang/dx-03/src/test/resources/programs/amb_queens.dx`

### Custom AD Primitives and Backward Rules

- [ad_custom_primitive.dx.example](ad_custom_primitive.dx.example)
  Source: `dx-03/src/test/resources/programs/smooth_primitive.dx`
- [ad_fused_backward.dx.example](ad_fused_backward.dx.example)
  Source: `dx-03/src/test/resources/programs/smooth_fused.dx`

### Advanced AD and Probabilistic Programming

- [ad_full_smooth_handler.dx.example](ad_full_smooth_handler.dx.example)
  Source: `dx-03/src/test/resources/programs/dx_grad_full.dx`
- [prob_bayesian_regression.dx.example](prob_bayesian_regression.dx.example)
  Source: `dx-03/src/test/resources/programs/prob_bayesian.dx`
- [prob_hmc_regression.dx.example](prob_hmc_regression.dx.example)
  Source: `dx-03/src/test/resources/programs/hmc.dx`
- [ad_ppl_combined.dx.example](ad_ppl_combined.dx.example)
  Source: synthetic — variational inference via gradient ascent on ELBO

### ML and Tensor Operations

- [ml_tensor_ops.dx.example](ml_tensor_ops.dx.example)
  Source: `dx-03/src/test/resources/programs/transformer_ops.dx`
- [ml_causal_attention.dx.example](ml_causal_attention.dx.example)
  Source: `dx-03/src/test/resources/programs/attention_forward.dx`
- [ml_mnist_training.dx.example](ml_mnist_training.dx.example)
  Source: `dx-03/src/test/resources/programs/mnist_dx_grad.dx`

### Typed Data and Python Bridge

- [typed_data_pipeline.dx.example](typed_data_pipeline.dx.example)
  Source: synthetic — schema-driven typed analysis flow
- [python_bridge_incremental.dx.example](python_bridge_incremental.dx.example)
  Source: synthetic — staged migration from Python to native DX

### LLM-First

- [llm_structured_output.dx.example](llm_structured_output.dx.example)
  Source: synthetic — typed schema-backed model output, not string parsing
- [llm_tool_workflow.dx.example](llm_tool_workflow.dx.example)
  Source: synthetic — typed tool functions with effect-tracked agent loop

### Edge / Embedded Inference

- [edge_sensor_inference.dx.example](edge_sensor_inference.dx.example)
  Source: synthetic — typed sensor input, bounded arena runtime, no Python at deployment
- [edge_quantized_pipeline.dx.example](edge_quantized_pipeline.dx.example)
  Source: synthetic — Int8 quantized inference, pool-backed allocation, native binary

## What To Read These Examples For

- `effects_run`: `never` and `once` handlers must remain first-class
- `effects_compose`: handler nesting order matters and must stay readable
- `async_no_coloring`: ordinary functions can perform effectful I/O without `async` function coloring
- `nested_fun_handle`: local functions inside handled regions must keep lexical scope and captured state
- `ad_scalar_grad`: grad() is a dx function via handle...with, not a compiler builtin
- `ad_custom_grad`: custom gradient handlers (tracing, clipping) use the same effect mechanism
- `prob_basic_inference`: probabilistic models are effectful functions; handlers implement inference
- `prob_monty_hall`: same effect pattern applies to simulation and classical probability
- `amb_basic`: multi-shot resume must remain a first-class target, not an afterthought
- `amb_collect`: handlers can branch and aggregate all outcomes, not just return a single result
- `amb_queens`: backtracking search is a flagship use-case for selective CPS and multi-shot effects
- `ad_full_smooth_handler`: the complete smooth handler surface — all differentiable ops in one handler
- `prob_bayesian_regression`: multi-parameter Bayesian inference with importance sampling
- `prob_hmc_regression`: THREE effects composing — smooth (AD) + random + state = HMC
- `ml_tensor_ops`: core tensor primitives (matmul, broadcasting, reductions, masking) must compose with AD
- `ml_causal_attention`: transformer attention is expressible as pure functions under smooth handlers
- `ad_ppl_combined`: AD + PPL compose — differentiating through a probabilistic scoring function for variational inference
- `ml_mnist_training`: end-to-end neural network training with effect-based AD is the flagship ML target
- `typed_data_pipeline`: schema-driven typed analysis replaces pandas/polars with compile-time safety
- `python_bridge_incremental`: staged Python migration — foreign bridge, not semantic host
- `llm_structured_output`: model output is typed data, not strings — schema-backed validation at the boundary
- `llm_tool_workflow`: tools are typed functions with effects — the model plans, DX orchestrates
- `edge_sensor_inference`: typed sensor input, arena-bounded runtime, no Python at deployment
- `edge_quantized_pipeline`: Int8 quantized inference with pool-backed allocation and native binary
- `ad_custom_primitive`: compile-time AD primitive discovery — no decorators, no registration
- `ad_fused_backward`: explicit backward rules for numerical stability — DX's custom_vjp
