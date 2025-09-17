##########################
# USER-CHANGEABLE VARIABLES

# Cargo profile: `dev` or `release`
CARGO_PROFILE ?= dev
ARTIFACTS_DIR ?= ./artifacts
PROFILE_DIR := $(if $(filter dev,$(CARGO_PROFILE)),debug,$(CARGO_PROFILE))
OVMF ?= /usr/share/ovmf/OVMF_CODE.fd
# values: false,true
QEMU_DISPLAY ?= true
# values: false, true
QEMU_KVM ?= false

##########################
# INTERNAL VARIABLES

# rustc 1.91: nightly needed as some unstable build features are required
RUSTUP_NIGHTLY_TOOLCHAIN=nightly-2025-09-07

KERNEL_COMMON_CARGO_ARGS = \
	--target ws/bins/kernel/x86_64-unknown-kernel.json \
	-Z build-std=core,alloc,compiler_builtins \
	-Z build-std-features=compiler-builtins-mem
UEFI_LOADER_COMMON_CARGO_ARGS = \
	--target x86_64-unknown-uefi

KERNEL_ARTIFACT = target/x86_64-unknown-kernel/$(PROFILE_DIR)/kernel
QEMU_ARG_ACCEL = $(if $(filter true,$(QEMU_KVM)),kvm,$(if $(filter false,$(QEMU_KVM)),tcg))
QEMU_BOOT_VOL = $(ARTIFACTS_DIR)/boot
QEMU_CPU_ARG = $(if $(filter true,$(QEMU_KVM)),host,$(if $(filter false,$(QEMU_KVM)),qemu64))
QEMU_ARG_DISPLAY = $(if $(filter true,$(QEMU_DISPLAY)),gtk,$(if $(filter false,$(QEMU_DISPLAY)),none))
QEMU_ARG_MONITOR = $(if $(filter true,$(QEMU_DISPLAY)),vc,$(if $(filter false,$(QEMU_DISPLAY)),none))
UEFI_LOADER_ARTIFACT = target/x86_64-unknown-uefi/$(PROFILE_DIR)/uefi-loader.efi

.PHONY: default
default: build

##########################
# BIN TARGETS

.PHONY: build
build: | check kernel uefi-loader ARTIFACTS_DIR
	ln -f -s ../$(KERNEL_ARTIFACT) $(ARTIFACTS_DIR)/kernel.elf64
	ln -f -s ../$(UEFI_LOADER_ARTIFACT) $(ARTIFACTS_DIR)
	cargo run --bin kernel-elf-checker -- $(ARTIFACTS_DIR)/kernel.elf64 >/dev/null


.PHONY: kernel
kernel:
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
	cargo build $(KERNEL_COMMON_CARGO_ARGS) \
		-p kernel \
		--profile $(CARGO_PROFILE) \
		--verbose

.PHONY: uefi-loader
uefi-loader:
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
	cargo build $(UEFI_LOADER_COMMON_CARGO_ARGS) \
		-p uefi-loader \
		--profile $(CARGO_PROFILE) \
		--verbose


.PHONY: check
check:
	cargo check --all-targets --all-features \
		-p kernel-lib \
		-p loader-lib \
		-p util
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
		cargo check -p kernel $(KERNEL_COMMON_CARGO_ARGS)
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
		cargo check -p uefi-loader $(UEFI_LOADER_COMMON_CARGO_ARGS)


.PHONY: clippy
clippy:
	cargo clippy --all-targets --all-features \
		-p kernel-lib \
		-p loader-lib \
		-p util
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
		cargo check --all-features -p kernel $(KERNEL_COMMON_CARGO_ARGS)
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
		cargo check --all-features -p uefi-loader $(UEFI_LOADER_COMMON_CARGO_ARGS)


.PHONY: doc
doc:
	cargo doc --no-deps --document-private-items \
		-p kernel-lib \
		-p loader-lib \
		-p util

.PHONY: fmt
fmt:
	@# We use some nightly features of rustfmt.
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) cargo fmt --all
	nix fmt


.PHONY: test
test:
	cargo test \
		-p kernel-lib \
		-p loader-lib \
		-p util


.PHONY: test-with-miri
test-with-miri:
	RUSTUP_TOOLCHAIN=$(RUSTUP_NIGHTLY_TOOLCHAIN) \
	cargo miri test \
		-p kernel-lib \
		-p loader-lib \
		-p util


.PHONY: qemu_integrationtest
qemu_integrationtest: | boot-vol
	@# QEMU/TCG allows better debuggability over QEMU/KVM
	qemu-system-x86_64 \
		-bios $(OVMF) \
		-cpu $(QEMU_CPU_ARG) \
		-debugcon file:debugcon.log \
		-display $(QEMU_ARG_DISPLAY) \
		-drive "format=raw,file=fat:rw:$(QEMU_BOOT_VOL)" \
		-m 512M \
		-machine q35,accel=$(QEMU_ARG_ACCEL) \
		-monitor $(QEMU_ARG_MONITOR) \
		-no-reboot \
		-nodefaults \
		-smp 4 \
		-vga std
		@# -d int


.PHONY: run
run: | qemu_integrationtest


.PHONY: boot-vol
boot-vol: | build
	mkdir -p $(QEMU_BOOT_VOL)/EFI/BOOT/
	cp $(ARTIFACTS_DIR)/kernel.elf64 $(ARTIFACTS_DIR)/boot
	cp $(ARTIFACTS_DIR)/uefi-loader.efi $(ARTIFACTS_DIR)/boot
	cp $(ARTIFACTS_DIR)/uefi-loader.efi $(ARTIFACTS_DIR)/boot/EFI/BOOT/BOOTX64.EFI

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
