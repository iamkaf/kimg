# kimg Plan

This file tracks only unfinished work.

Public-facing roadmap items live in [README.md](README.md).

## Release and Packaging

- [ ] Verify `dist/` can be packed and published cleanly from CI as well as locally
- [ ] Add a local `npm pack` install smoke test for both Node.js and browser consumption
- [ ] Add CI coverage for:
  - `cargo test --workspace`
  - `cargo check --target wasm32-unknown-unknown -p kimg-wasm`
  - `./scripts/build.sh`
  - `cargo audit`
  - `cargo deny check`

## Deferred Technical Evaluation

- [ ] Revisit PSD parser replacement with `rawpsd` only if PSD import becomes important again
- [ ] Evaluate `cosmic-text` only when the text roadmap item becomes active
