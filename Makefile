# Cargo profile: `dev` or `release`
CARGO_PROFILE ?= dev
ARTIFACTS_DIR ?= ./artifacts

PROFILE_DIR := $(if $(filter dev,$(CARGO_PROFILE)),debug,$(CARGO_PROFILE))

KERNEL_COMMON_CARGO_ARGS = \
	--target kernel/x86_64-unknown-kernel.json \
	-Z build-std=core,alloc,compiler_builtins \
	-Z build-std-features=compiler-builtins-mem

UEFI_LOADER_COMMON_CARGO_ARGS = \
	--target x86_64-unknown-uefi

.PHONY: default
default: build

##########################
# BIN TARGETS

.PHONY: build
build: kernel uefi-loader | ARTIFACTS_DIR
	ln -f -s ../target/x86_64-unknown-uefi/$(PROFILE_DIR)/uefi-loader.efi $(ARTIFACTS_DIR)
	ln -f -s ../target/x86_64-unknown-kernel/$(PROFILE_DIR)/kernel $(ARTIFACTS_DIR)/kernel.elf64

.PHONY: kernel
kernel:
	cargo build $(KERNEL_COMMON_CARGO_ARGS) \
		-p kernel \
		--profile $(CARGO_PROFILE) \
		--verbose

.PHONY: uefi-loader
uefi-loader:
	cargo build $(UEFI_LOADER_COMMON_CARGO_ARGS)\
		-p uefi-loader \
		--profile $(CARGO_PROFILE) \
		--verbose

.PHONY: check
check:
	cargo check \
		-p kernel-lib \
		-p uefi-loader-lib
	cargo check -p kernel $(KERNEL_COMMON_CARGO_ARGS)
	cargo check -p uefi-loader $(UEFI_LOADER_COMMON_CARGO_ARGS)

.PHONY: test
test:
	cargo test \
		-p kernel-lib \
		-p uefi-loader-lib


.PHONY: doc
doc:
	cargo doc --no-deps --document-private-items \
		-p kernel-lib \
		-p uefi-loader-lib

##########################
# HELPER TARGETS

.PHONY: clean
clean:
	cargo clean
	rm -rf $(ARTIFACTS_DIR)


##########################
# MISC TARGETS

ARTIFACTS_DIR:
	mkdir -p ./artifacts
