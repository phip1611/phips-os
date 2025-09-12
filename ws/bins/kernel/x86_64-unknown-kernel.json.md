# Custom `x86_64-unknown-kernel` Compiler Target

## Steps to Reproduce
1. Get the base target spec: `rustc -Z unstable-options --print target-spec-json --target x86_64-unknown-none`
2. Set relocation model to static and remove all PIC/PIE options
