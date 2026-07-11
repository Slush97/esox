# Phase 0 Baseline

Measured on 2026-07-10 from the uncommitted Phase 0 working tree based on commit
`dc231156a90a161164765f5af142dbaf4d40b90e`. These numbers are a comparison
point, not performance guarantees.

## Host

- Linux 7.1.2-arch3-1, x86-64
- Intel Core Ultra 7 155H, 22 logical CPUs available
- 14 GiB RAM
- Wayland session (Hyprland/KDE environment)
- Rust 1.95.0 and Cargo 1.95.0
- Release profile: `opt-level = "s"`, thin LTO, one codegen unit, stripped

The machine was running a normal desktop session. No attempt was made to pin CPU
frequency, isolate cores, or stop unrelated processes, so later comparisons
should use multiple samples before attributing small changes to Esox.

## Build and binary

| Measurement | Result |
| --- | ---: |
| Cold release build, demo | 66.77 s wall, 515.87 s user, 33.63 s system |
| No-change release rebuild, demo | 0.29 s wall |
| Stripped demo binary | 9,430,136 bytes (9.0 MiB) |

The cold build used an empty, isolated target directory:

```sh
CARGO_TARGET_DIR=/tmp/esox-phase0-baseline cargo build --release -p demo
```

The binary is a dynamically linked x86-64 ELF. Its direct dynamic dependencies
on this host are `libgcc_s`, `libm`, and `libc`; runtime operation additionally
uses the system windowing, Vulkan, and fontconfig facilities described in the
README.

## Tests

| Configuration | Passed | Ignored | Warm wall time |
| --- | ---: | ---: | ---: |
| Workspace defaults | 400 | 36 | 8.99 s |
| Workspace all features | 602 | 37 | 7.24 s |

Commands:

```sh
cargo test --workspace --quiet
cargo test --workspace --all-features --quiet
```

The all-features run was faster because compilation work from the preceding
default run was already cached. The times therefore describe these local
validation runs, not clean test-build performance.

## Startup and idle runtime

The stripped release demo was started from the isolated target directory and
left untouched for 30 seconds. Startup was measured from process launch until
the compositor first listed the application's window. The window was then
closed through the compositor so Esox wrote its normal performance report.

| Measurement | Result |
| --- | ---: |
| Window visible | 238 ms after process launch |
| Session duration | 30.3 s |
| Frames rendered | 1,806 (59.7 FPS) |
| Frames skipped | 0 |
| Average / peak process CPU | 9.5% / 99.1% |
| Average CPU frame time | 16.587 ms |
| Median / p95 / p99 CPU frame time | 16.608 / 16.956 / 17.193 ms |
| Maximum CPU frame time | 23.494 ms |
| Jank frames over 33.22 ms | 0 |
| Final / peak RSS | 79.6 / 79.6 MiB |
| Final virtual memory | 766 MiB |
| Final proportional set size | 52.5 MiB |

The current frame-skip optimization is disabled, so this "idle" sample renders
continuously at the display refresh rate. CPU and frame-time values include the
startup portion of the session. This is the behavior later power and damage
tracking work should compare against; it is not an idle-efficiency target.

## Reproduction notes

- Use the same pinned toolchain and release profile.
- Use an empty target directory for a cold build and retain it for the no-change
  rebuild.
- Record whether dependency artifacts or compiler caches are reused.
- Run the demo without interaction for at least 30 seconds on a 60 Hz display.
- Record the compositor, GPU/driver, display refresh rate, and whether frame
  skipping is enabled when comparing runtime results.
