# Roadmap

## Phase 1 — Accessibility & Internationalization

The differentiator. No Linux toolkit does a11y well. esox will.

- [ ] Complete AT-SPI2 event bridge (focus, key events, value changes, live regions)
- [ ] Keyboard navigation contract for every widget (Tab order, arrow keys, Enter/Space)
- [ ] Screen reader testing with Orca
- [ ] High-contrast theme
- [ ] Focus-visible indicators on all interactive widgets
- [ ] RTL layout support
- [ ] Bidirectional text rendering (UAX #9)
- [ ] ICU-aware line breaking (UAX #14)
- [ ] Locale-aware number/date formatting hooks

## Phase 2 — Modern UX & Polish

Close the gap with contemporary design systems.

- [ ] Animation overhaul: spring physics, interruptible animations, staggered sequences, gesture-driven
- [ ] Design tokens: elevation/shadows, typography scale, spacing grid (4px/8px), motion tokens
- [ ] Subpixel text rendering (LCD filtering for non-HiDPI)
- [ ] New widgets: color picker, date/time picker, command palette, breadcrumbs, skeleton loaders, sheet/drawer, rich tooltips, searchable lists
- [ ] Smooth window resize
- [ ] Multi-window support

## Phase 3 — Developer Experience

Make it easy to build with.

- [ ] Documentation site
- [ ] Interactive widget gallery
- [ ] `cargo generate` app template
- [ ] Hot-reload for themes
- [ ] DevTools overlay: layout inspector, a11y tree viewer, perf flamegraph
- [ ] Migration guides from GTK/Qt

## Phase 4 — Ecosystem & Community

- [ ] Publish to crates.io
- [ ] CI/CD (GitHub Actions: build, test, clippy)
- [ ] Benchmark suite (frame times, memory, startup, binary size)
- [ ] Example apps: settings panel, file manager, text editor
- [ ] XDG integration guide
- [ ] Wayland-only minimal builds

## Stretch

- [ ] GPU text rendering (Vello-style)
- [ ] WebAssembly target
- [ ] CSS-like theming engine
- [ ] Visual UI builder
