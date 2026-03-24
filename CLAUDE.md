# CLAUDE.md

## Project Overview

Esox is a GPU-accelerated, immediate-mode UI toolkit for native Linux applications, written in Rust. It targets small binaries (~8MB), zero runtime dependencies, and first-class accessibility.

## Workspace Structure

- `crates/esox_ui` — Widget library, layout engine, theming
- `crates/esox_gfx` — GPU rendering (wgpu/Vulkan), shaders, atlas, damage tracking
- `crates/esox_font` — Font loading (ttf-parser), shaping (rustybuzz), rasterization (swash)
- `crates/esox_platform` — Windowing (winit), input, clipboard, AT-SPI2 a11y bridge
- `crates/esox_input` — Platform-independent input types
- `examples/` — demo, layout_showcase, material_showcase

## Development Commands

```sh
# Build everything
cargo build --workspace

# Release build (optimized, stripped)
cargo build --workspace --release

# Run examples
cargo run -p demo --release
cargo run -p layout_showcase --release
cargo run -p material_showcase --release

# Check formatting (must pass before commit)
cargo fmt --all -- --check

# Format code
cargo fmt --all

# Lint (must pass before commit, warnings are errors in CI)
cargo clippy --workspace -- -D warnings

# Run tests
cargo test --workspace
```

## Code Standards

- **Format before commit**: Always run `cargo fmt --all` before committing.
- **Clippy clean**: Fix all clippy warnings. Use `#[allow(clippy::...)]` only with a comment explaining why.
- **No unsafe**: Avoid `unsafe` unless absolutely necessary for FFI or performance-critical paths. Always document why.
- **Error handling**: Use `thiserror` for library errors. No `.unwrap()` in library code — only in examples and tests.
- **Edition 2024**: This project uses Rust 2024 edition features.

## Architecture Notes

- **Immediate-mode**: Widgets are method calls on `Ui` returning `Response`. All mutable state lives in the application. The library stores nothing between frames except layout cache and scroll offsets.
- **Layout tree**: Built each frame, solved after `finish()`, cached for next frame's `prev_layout` lookups. Two-pass: measure (bottom-up) then arrange (top-down).
- **Render pipeline**: Widgets emit `QuadInstance`s into a `Frame`. Platform submits them to wgpu as instanced draws.
- **Damage tracking**: `DamageTracker` + optional `TileGrid` for partial redraw. Input events call `damage.invalidate_all()`. Currently the frame-skip optimization is disabled (was causing blank screens).

## Git Practices

- Branch from `main` for all work
- Write clear commit messages: imperative mood, explain the "why"
- One concern per commit — don't mix unrelated changes
- Run `cargo fmt --all && cargo clippy --workspace` before every commit
- Don't commit generated files, build artifacts, or editor configs
