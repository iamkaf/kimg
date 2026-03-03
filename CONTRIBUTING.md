# Contributing to kimg

Thanks for your interest in contributing. Here's how to get set up and what to expect.

## Prerequisites

- Node.js and npm
- [Rust](https://rustup.rs/) (stable)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- `wasm-bindgen-cli`: `cargo install wasm-bindgen-cli`
- (Optional) `wasm-opt` from [binaryen](https://github.com/WebAssembly/binaryen) for smaller builds

## Getting started

```bash
git clone https://github.com/iamkaf/kimg.git
cd kimg
npm install
cargo test -p kimg-core
```

This runs all core tests. They should pass in under a second.

To build the WASM output:

```bash
./scripts/build.sh
```

The tracked JS/TS wrapper sources live in `js/`, the npm package metadata lives at the repo root, and `./scripts/build.sh` emits the consumable package into `dist/` alongside the generated wasm-bindgen output. The demo page at `demo/index.html` loads from `dist/`. Serve it with any static file server:

```bash
./scripts/demo.sh
```

## Making changes

1. Fork the repo and create a branch from `main`.
2. Make your changes. Add tests for new functionality.
3. Run `npm run test:all` and `cargo clippy --workspace` to check for issues.
   Run `npm run test:demo` too if you changed the browser demo or visual test suite.
   Run `npm run test:pack` too if you changed packaging, exports, or the build pipeline.
4. Open a pull request.

## Code style

- Run `cargo fmt` before committing.
- Run `npm run fmt:js` before committing if you touched the TS wrapper or tests.
- Keep dependencies minimal. The WASM binary size matters.
- Tests go in `#[cfg(test)] mod tests` blocks within each source file.
- Prefer concrete assertions over fuzzy ranges in tests when possible.

## What to work on

Check the open issues for current priorities. Docs, benchmarks, SIMD optimization, and fuzz testing are areas where help is most useful right now.

If you're unsure about something, open an issue first.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
