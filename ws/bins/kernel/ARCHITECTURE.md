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
