.PHONY: demo-emit demo-verify demo-plan runtime-stub-info runtime-stub-plan runtime-stub-build-plan

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

runtime-stub-info:
	cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-info

runtime-stub-plan:
	cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-plan -- $(OBJ_OUT) $(EXE_OUT)

runtime-stub-build-plan:
	cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan -- $(RUNTIME_PROFILE) $(RUNTIME_TARGET_DIR)
