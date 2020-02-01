# npm-rs

A build-time dependency for Cargo build scripts to assist in invoking npm scripts.

**!!!** This crate is currently very young and naively written. Please check the code thoroughly before even thinking about using it in anything important **!!!**

## Example

### Build a local npm project inside the cargo target directory

This example copies the relevant files in your project directory to `$OUT_DIR/npm_dir` and builds the npm project there.

```toml
# Cargo.toml

# ...
[build-dependencies]
npm-rs = {git = "https://github.com/peacememories/npm-rs"}
# ...
```

```rust
// build.rs

use npm_rs::Build;
use std::env;
use std::path::PathBuf;

fn main() {
    Build::new()
        .project_directory(env::var("CARGO_MANIFEST_DIR").unwrap())
        .target_directory(PathBuf::from(env::var("OUT_DIR").unwrap()).join("npm_dir"))
        .copy_all()
        .run_script("build");
}

```

## Feedback & Contributions

Feedback and Contributions are welcome, either through GitHubs issue and pr tracker or on Riot/IRC under the handle `@peacememories`

Please respect the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)
