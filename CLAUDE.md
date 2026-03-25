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

### Branching
- Branch from `main` for all work: `feature/<name>`, `fix/<name>`, `refactor/<name>`
- Keep branches focused — one feature or fix per branch
- Rebase onto `main` before opening a PR to keep history linear

### Commits
- **Atomic commits**: One logical change per commit — don't mix unrelated changes
- **Imperative mood**: "Add contrast utility" not "Added contrast utility"
- **Explain the why**: The diff shows *what* changed; the message explains *why*
- **Format**: Short subject line (<72 chars), blank line, then body if needed
- **Pre-commit checks**: Run `cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace` before every commit
- Don't commit generated files, build artifacts, editor configs, or `.env` files

### Commit Message Format
```
<type>: <short summary>

<optional body — explain motivation, trade-offs, what was considered>
```

Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `chore`, `ci`

Examples:
```
feat: add WCAG contrast ratio utility to theme module
fix: re-enable frame-skip when no damage detected
perf: shrink text QuadInstance path to 80 bytes
refactor: extract glyph batching into separate pass
```

### Pull Requests
- PR title matches the primary commit type and summary
- Description includes: what changed, why, and how to test
- Link related issues
- Keep PRs small enough to review in one sitting — split large work into stacked PRs if needed

### Code Review
- All changes to `main` go through PR review
- CI must pass (fmt, clippy, tests) before merge
- Squash-merge feature branches to keep `main` history clean
