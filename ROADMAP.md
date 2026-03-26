# Roadmap

## Strategic Vision

Two differentiators: (1) the best a11y story on Linux, and (2) the first UI toolkit with a first-class AI generation story — a declarative layer where AI describes *what* it wants and the runtime handles *how*.

## Phase 1 — Accessibility & Internationalization

No Linux toolkit does a11y well. Esox will.

- [ ] Complete AT-SPI2 event bridge (focus, key events, value changes, live regions)
- [ ] Keyboard navigation contract for every widget (Tab order, arrow keys, Enter/Space)
- [ ] Screen reader testing with Orca
- [x] High-contrast theme
- [ ] Focus-visible indicators on all interactive widgets
- [ ] RTL layout support
- [ ] Bidirectional text rendering (UAX #9)
- [ ] ICU-aware line breaking (UAX #14)
- [ ] Locale-aware number/date formatting hooks

## Phase 2 — Declarative UI Layer (AI Bridge)

The keystone for AI-assisted UI generation.

### 2.1 Description Format
- [ ] Data-driven UI representation (Rust DSL + serde-compatible for JSON/TOML)
- [ ] Interpreter: walk description tree → `Ui` method calls
- [ ] Validator: catch structural errors before rendering
- [ ] Support all 45+ existing widgets, flex/grid layout, style overrides

### 2.2 Semantic Layout Primitives
- [ ] `PageLayout` (header / sidebar / content / footer regions)
- [ ] `SidebarLayout` (collapsible side + main)
- [ ] `MasterDetail` (list + detail pane)
- [ ] `DashboardGrid` (responsive card grid)
- [ ] `FormLayout` (auto label alignment, responsive stacking)

### 2.3 Semantic Styling Tokens
- [ ] `Emphasis` levels (prominent, subtle, default)
- [ ] `Density` levels (compact, comfortable, spacious)
- [ ] `SurfaceLevel` (base, raised, overlay, scrim)
- [ ] Theme generation from seed color (Material You–style algorithm)

## Phase 3 — Visual Polish

CSS gaps that the declarative format will expose. Build as needed.

### Layout
- [ ] Absolute / fixed / sticky positioning
- [ ] Overflow control per-axis (hidden / scroll / auto)
- [ ] Sticky scroll headers

### Animation
- [ ] Keyframe animations (multi-step, named, reusable)
- [ ] Layout animations (auto-animate size/position changes)
- [ ] Spring physics, interruptible animations
- [ ] Staggered sequences (delay per child index)

### Visual Effects
- [ ] Backdrop blur / glassmorphism
- [ ] SVG / vector icon rendering
- [ ] Inset shadows, multiple box shadows
- [ ] Element-level filters (blur, grayscale, brightness)

### Typography
- [ ] Variable font axes (continuous weight/width/slant)
- [ ] Multi-line text ellipsis (`-webkit-line-clamp` equivalent)
- [ ] `font-variant-numeric` (tabular figures)
- [ ] Subpixel text rendering (LCD filtering for non-HiDPI)

### Design System
- [ ] Style cascade / inheritance (child inherits parent text color)
- [ ] Design tokens with semantic layers (reference → system → component)
- [ ] Component variants (filled, outlined, text, tonal, elevated)

## Phase 4 — AI Feedback Loop

- [ ] Headless rendering (wgpu render-to-texture → PNG)
- [ ] Layout annotation overlay (bounding boxes + spacing values on screenshot)
- [ ] Screenshot → AI → updated description round-trip
- [ ] Template library (login, settings, dashboard, data table patterns)

## Phase 5 — Professional Components

- [ ] Date/time picker, calendar view
- [ ] Command palette (Ctrl+K)
- [ ] Autocomplete / typeahead
- [ ] Data grid (sort, filter, group, inline edit)
- [ ] Virtualized 2D grid
- [ ] Color picker
- [ ] Rich text editor
- [ ] Timeline, carousel
- [ ] Smooth window resize
- [ ] Multi-window support

## Phase 6 — Developer Experience & Ecosystem

- [ ] Documentation site
- [ ] Interactive widget gallery / storybook
- [ ] DevTools overlay: layout inspector, a11y tree viewer, perf flamegraph
- [ ] Hot-reload for description format + themes
- [ ] `cargo generate` app template
- [ ] Publish to crates.io
- [ ] CI/CD (GitHub Actions: build, test, clippy)
- [ ] Benchmark suite (frame times, memory, startup, binary size)
- [ ] Example apps: settings panel, file manager, text editor

## Stretch

- [ ] GPU text rendering (Vello-style)
- [ ] WebAssembly target
- [ ] Visual UI builder (dogfooded on Esox itself)
- [ ] Design token export/import (Figma tokens, Style Dictionary)

## Principles

- **Let the format drive features**: if you can't express a common pattern, that's the next thing to build
- **Incremental value**: each phase is useful on its own
- **AI-first doesn't mean AI-only**: the declarative format should also be great for humans
- **Don't over-plan**: build, discover, iterate
