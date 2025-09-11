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

KERNEL_COMMON_CARGO_ARGS = \
	--target kernel/x86_64-unknown-kernel.json \
	-Z build-std=core,alloc,compiler_builtins \
	-Z build-std-features=compiler-builtins-mem
UEFI_LOADER_COMMON_CARGO_ARGS = \
	--target x86_64-unknown-uefi

KERNEL_ARTIFACT = target/x86_64-unknown-kernel/$(PROFILE_DIR)/kernel
UEFI_LOADER_ARTIFACT = target/x86_64-unknown-uefi/$(PROFILE_DIR)/uefi-loader.efi
QEMU_BOOT_VOL = $(ARTIFACTS_DIR)/boot
QEMU_ACCEL_ARG = $(if $(filter true,$(QEMU_KVM)),kvm,$(if $(filter false,$(QEMU_KVM)),tcg))
QEMU_CPU_ARG = $(if $(filter true,$(QEMU_KVM)),host,$(if $(filter false,$(QEMU_KVM)),qemu64))
QEMU_DISPLAY_ARG = $(if $(filter true,$(QEMU_DISPLAY)),gtk,$(if $(filter false,$(QEMU_DISPLAY)),none))
QEMU_MONITOR_ARG = $(if $(filter true,$(QEMU_DISPLAY)),vc,$(if $(filter false,$(QEMU_DISPLAY)),none))

.PHONY: default
default: build

##########################
# BIN TARGETS

.PHONY: build
build: | kernel uefi-loader ARTIFACTS_DIR
	ln -f -s ../$(KERNEL_ARTIFACT) $(ARTIFACTS_DIR)/kernel.elf64
	ln -f -s ../$(UEFI_LOADER_ARTIFACT) $(ARTIFACTS_DIR)


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
		-p uefi-loader-lib \
		-p util
	cargo check -p kernel $(KERNEL_COMMON_CARGO_ARGS)
	cargo check -p uefi-loader $(UEFI_LOADER_COMMON_CARGO_ARGS)


.PHONY: clippy
clippy:
	cargo clippy --all-targets --all-features \
		-p kernel-lib \
		-p uefi-loader-lib \
		-p util
	cargo check --all-features -p kernel $(KERNEL_COMMON_CARGO_ARGS)
	cargo check --all-features -p uefi-loader $(UEFI_LOADER_COMMON_CARGO_ARGS)


.PHONY: doc
doc:
	cargo doc --no-deps --document-private-items \
		-p kernel-lib \
		-p uefi-loader-lib \
		-p util

.PHONY: fmt
fmt:
	cargo fmt --all
	nix fmt


.PHONY: test
test:
	cargo test \
		-p kernel-lib \
		-p uefi-loader-lib \
		-p util


.PHONY: qemu_integrationtest
qemu_integrationtest: | boot-vol
	# We don't use KVM here as QEMU better allows to debug issues.
	qemu-system-x86_64 \
		-bios $(OVMF) \
		-cpu $(QEMU_CPU_ARG) \
		-display $(QEMU_DISPLAY_ARG) \
		-drive "format=raw,file=fat:rw:$(QEMU_BOOT_VOL)" \
		-m 512M \
		-machine q35,accel=$(QEMU_ACCEL_ARG) \
		-monitor $(QEMU_MONITOR_ARG) \
		-no-reboot \
		-nodefaults \
		-smp 4 \
		-vga std
		# -d int


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
