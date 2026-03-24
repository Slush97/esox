# Contributing to esox

Thanks for your interest in contributing.

## Getting started

1. Fork and clone the repo
2. Make sure you can build: `cargo build --release`
3. Run the demo: `cargo run -p demo --release`

## Development

```sh
# check everything compiles
cargo check --workspace

# run clippy
cargo clippy --workspace

# format
cargo fmt --all
```

## Submitting changes

- Open an issue first for anything non-trivial so we can discuss the approach
- Keep PRs focused — one concern per PR
- Make sure `cargo clippy --workspace` is clean
- Add or update examples if you're adding widgets or changing the API

## What to work on

Check the [roadmap](ROADMAP.md) and open issues. Accessibility work (Phase 1) is the current priority.

## License

By contributing, you agree that your contributions will be dual-licensed under MIT and Apache 2.0.
