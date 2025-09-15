# PhipsOS Kernel

## Booting / Loading

The kernel expects to be loaded in 64-bit mode on the bootstrap processor (BSP).
All the application processors (AP) remain in wait state until the kernel
awakens them eventually.

Further, the following properties apply or must be tree:

- the OS loader must pass a valid PhipsOS boot information
- the kernel set's up its own stack
- in case of UEFI, the boot services must have been exited already
- the page table with that the kernel was loaded will likely be discarded

## Communication with Outer World

The following output formats (e.g., to get logs) are supported:

- debugcon log
- serial log

## Subsystem Initialization Order

1. Early Init (no allocations)
  - Logging
1. Init
  - kernel heap
  - more sophisticated logging
1. Wake up APs (all CPUs will execute the same setup from here)
1. Set MSRs and CR registers as needed

## Memory Management

### Kernel Heap

The kernel heap is also the heap from that the `alloc` crate, thus `Vec`, `Box`,
etc. allocate. To keep things simple, this is built into the kernel ELF itself.
When the loader properly loads the kernel's LOAD segments, the heap will be
statically available.

The advantage of that approach is that we can use allocations right from the
beginning and do not have to parse the memory map and find a suitable memory
region.
