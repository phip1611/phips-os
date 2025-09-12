# Cargo workspace of PhipsOS

## Directory Structure

Although untypical for normal Rust projects, this is a project that builds
no_std bins. Experience and personal taste shows that the separation into
`./bins` for `no_std` binaries for non-standard targets and a `./libs` directory
with crates that also build + run on the build host gives a much better
developer and IDE experience.
