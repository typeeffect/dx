.PHONY: \
	demo-emit \
	demo-verify \
	demo-plan \
	demo-build \
	demo-build-verify \
	prove-subset \
	prove-subset-verify \
	prove-subset-dry-run \
	prove-runnable \
	prove-runnable-verify \
	runtime-stub-info \
	runtime-stub-plan \
	runtime-stub-build-plan

DEMO ?= examples/backend/closure_call_int.dx
BUILD_DIR ?= build
DEMO_NAME := $(notdir $(basename $(DEMO)))
LLVM_OUT ?= $(BUILD_DIR)/$(DEMO_NAME).ll
OBJ_OUT ?= $(BUILD_DIR)/$(DEMO_NAME).o
EXE_OUT ?= $(BUILD_DIR)/$(DEMO_NAME)
RUNTIME_PROFILE ?= debug
RUNTIME_TARGET_DIR ?=

demo-emit:
	cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- $(DEMO) $(LLVM_OUT)

demo-verify:
	cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- --verify $(DEMO) $(LLVM_OUT)

demo-plan:
	cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- $(DEMO) $(BUILD_DIR)

demo-build:
	scripts/build_backend_demo.sh $(DEMO) $(BUILD_DIR)

demo-build-verify:
	scripts/build_backend_demo.sh --verify $(DEMO) $(BUILD_DIR)

prove-subset:
	scripts/prove_backend_subset.sh

prove-subset-verify:
	scripts/prove_backend_subset.sh --verify

prove-subset-dry-run:
	scripts/prove_backend_subset.sh --dry-run

prove-runnable:
	scripts/prove_executable_entry_subset.sh

prove-runnable-verify:
	scripts/prove_executable_entry_subset.sh --verify

runtime-stub-info:
	cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-info

runtime-stub-plan:
	cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-plan -- $(OBJ_OUT) $(EXE_OUT)

runtime-stub-build-plan:
	cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan -- $(RUNTIME_PROFILE) $(RUNTIME_TARGET_DIR)
