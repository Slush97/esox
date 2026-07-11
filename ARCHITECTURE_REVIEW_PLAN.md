# Esox Architecture Review and Stabilization Plan

**Status:** Proposed (revision 2)
**Audience:** Maintainers and architecture reviewers
**Scope:** Framework foundations required before broader widget development or a public release
**Last updated:** 2026-07-10

## 1. Purpose

Esox has credible rendering, layout, text, platform, and widget foundations, but several contracts are not yet strong enough for downstream applications to depend on. This plan defines the work needed to move Esox from a capable prototype to a dependable Linux UI framework.

The plan prioritizes architectural correctness before API stabilization, visual polish, additional widgets, or crates.io publication. It is intentionally narrower than `ROADMAP.md`: this document is a stabilization program with review gates and measurable exit criteria.

## 2. Outcome

At the end of this plan, Esox should provide these guarantees:

1. A frame paints, hit-tests, damages, and exposes accessibility bounds from the same solved geometry.
2. UI construction and layout can run without a window or GPU.
3. Logical and physical coordinate spaces are explicit and consistently converted.
4. Widget identity composes safely across reusable components, lists, overlays, and windows.
5. Keyboard, pointer, touch, IME, and accessibility interactions follow documented contracts.
6. Text editing respects Unicode grapheme, word, line-breaking, and bidirectional behavior.
7. Every supported feature combination compiles and the core configurations run in CI.
8. The intended public API is documented, versioned, and every public item is deliberately categorized as stable, experimental, or internal.
9. A downstream application can use Esox without relying on implementation modules.

## 3. Current Assessment

### Strengths to preserve

- Clear top-level intent across `esox_ui`, `esox_gfx`, `esox_font`, `esox_input`, and `esox_platform`.
- Rust-native rendering and text stack using WGPU, rustybuzz, and swash.
- Immediate-mode application ergonomics with explicit application-owned state.
- Substantial unit coverage for layout algorithms, rendering data structures, text internals, markup, and animation.
- Existing investment in damage tracking, portals, IME events, keyboard navigation, and semantic widget metadata.
- CI already expresses the expected baseline: formatting, warnings-as-errors, tests, and release build.

### Confirmed gaps

| Priority | Gap | Consequence |
| --- | --- | --- |
| P0 | Current layout is solved after widgets paint and is consumed on the next frame | Incorrect first frame and stale geometry after structural or dimensional changes |
| P0 | AT-SPI bridge discards snapshots and the semantic tree is effectively flat | Accessibility is not functional despite being a strategic differentiator |
| P0 | UI construction directly requires GPU resources | Headless testing, alternate renderers, deterministic layout tests, and software fallback are difficult |
| P1 | Input model is mouse-oriented and loses scroll/touch information | Incomplete touchpad, touchscreen, pointer capture, and gesture behavior |
| P1 | Text navigation and wrapping are not Unicode-conformant | Broken editing for combining marks, emoji sequences, RTL text, and many languages |
| P1 | Logical and physical pixels are not consistently represented | DPI-dependent layout and input bugs remain likely |
| P1 | Focus traversal is global, including while a modal is open | Modal interaction and keyboard accessibility are incorrect |
| P1 | Platform runtime owns one window and ignores event `WindowId` | Multi-window support requires architectural change |
| P1 | Optional feature configurations are not kept green | `a11y` does not compile today; `mesh3d` and `markup` compile but regress silently without CI coverage |
| P2 | Public API surface is broad and exposes implementation modules | Refactoring becomes a breaking change and documentation is hard to complete |
| P2 | No complete widget, GPU, accessibility, or visual regression suite | Unit tests do not protect user-visible framework behavior |
| P2 | 3D rendering shares the core 2D graphics crate | Compile cost, ownership, and stability boundaries are unnecessarily coupled |

## 4. Golden Framework Rules

These are review criteria, not aspirations.

### 4.1 One committed geometry per frame

Measurement, layout, painting, hit testing, clipping, accessibility bounds, and damage tracking must agree. A component must never paint from one frame's geometry while input or semantics use another frame's geometry.

### 4.2 Semantics are part of the widget contract

Every interactive widget must define its role, accessible name, state, value, actions, relationships, focus behavior, and relevant events. Accessibility cannot be reconstructed reliably from paint primitives after the fact.

### 4.3 Platform, UI core, and renderer have separate ownership

- The platform layer owns windows, event-loop integration, portals, clipboard, and surface lifecycle.
- The UI core owns identity, state reconciliation, layout, interaction, focus, and semantics.
- A renderer consumes a resolved display list and uploads resources.
- Text shaping and editing must not require a GPU atlas.

### 4.4 Coordinate spaces are explicit

Logical points, physical pixels, local coordinates, window coordinates, and transformed coordinates must not be interchangeable `f32` values without a named conversion boundary.

### 4.5 Identity is hierarchical and stable

Reusable components must be able to scope child IDs. Reordering siblings must not transfer state unexpectedly. Duplicate IDs should be detected in debug builds.

### 4.6 Input is device-independent

Mouse, pen, touch, touchpad, keyboard, accessibility actions, and synthetic input should enter a shared routing model with capture, targeting, bubbling or equivalent propagation, and cancellation semantics.

### 4.7 Unicode behavior is delegated to conformant algorithms

Grapheme segmentation, word boundaries, line breaking, bidirectional ordering, script runs, font fallback, cursor mapping, and selection should use proven libraries or Unicode conformance data rather than local heuristics.

### 4.8 Supported configurations stay green

A feature is not supported unless it compiles and is exercised in CI. A documented capability is not complete until a contract test validates it.

### 4.9 Public API is deliberate

Implementation modules remain private by default. Public types have documented ownership, threading, error, panic, coordinate, and lifecycle contracts.

## 5. Work Policy During Stabilization

Until Gate 2 is approved:

- Do not add new widgets except those required to validate an architectural decision.
- Do not publish the crates or promise API stability.
- Do not add styling systems that depend on current layout internals.
- Do not begin `ADR-001` or any Phase 1+ restructuring before Gate 0.5 records the spike outcomes.
- Accept bug fixes, tests, documentation corrections, and narrowly scoped performance fixes.
- Accept declarative description-format prototyping that consumes the display-list and semantic-tree APIs under design; it validates them from the consumer's side and serves the strategic direction (§6).
- Keep architecture changes split into reviewable commits with an ADR for each durable decision.

## 6. Sizing, Milestones, and Strategic Alignment

### Sizing

Rough solo-effort estimates, assuming the Gate 0.5 spikes favor adoption over in-house builds. Where a spike leads to building in-house, the affected phase grows substantially.

| Phase | Estimate | Notes |
| --- | --- | --- |
| 0 — Baseline | 1–2 weeks | Mechanical; the clippy and `a11y` fixes are known quantities |
| 0.5 — Decision spikes | 3–5 weeks | Four one-week spikes plus write-ups |
| 1 — Frame contract | 4–8 weeks | Upper end if the layout solver is redesigned in-house |
| 2 — Core boundaries | 8–12 weeks | Largest single restructuring |
| 3 — Coordinates, identity, focus | 4–6 weeks | |
| 4 — Accessibility | 6–10 weeks | Lower end if AccessKit is adopted |
| 5 — Input and text | 8–12 weeks | Lower end if Parley or cosmic-text is adopted |
| 6 — Regression infrastructure | 4–6 weeks initial | Then ongoing maintenance |
| 7 — API and release | 3–4 weeks | |

Total: roughly 10–16 months of focused solo work. This is stated so scope decisions are made deliberately up front — cutting a phase's scope is acceptable; discovering the total a year in is not.

### Mid-program release

An `0.x` experimental release ships after Gate 3, not Gate 7. It includes the current-frame layout contract, headless construction, typed coordinates, hierarchical identity, and focus scopes, published with an explicit "experimental, expect breakage" banner and no semver promise. Waiting for Gate 7 means more than a year with no external users, no feedback, and no forcing function on API quality.

### Strategic alignment

Esox's stated differentiator is an AI-targetable declarative description layer backed by headless rendering and a screenshot feedback loop. This plan is the prerequisite work for that layer, and two of its artifacts serve it directly:

- The Phase 2 display list and semantic tree must be serializable structured data — the same property snapshot testing needs.
- Headless construction (Gate 2) is the substrate for AI-driven generate–render–inspect loops.

Phase 2 design work must name the declarative layer as a consumer so these properties are not retrofitted later.

## 7. Phase 0: Restore and Record the Baseline

**Goal:** Make the current support claims reproducible before restructuring them.

### Deliverables

- Fix default `clippy -D warnings` failures on the pinned toolchain.
- Fix compilation of the `a11y` configuration (two compile errors today). `mesh3d` and `markup` currently compile; the CI matrix below keeps every optional configuration green.
- Define the supported feature matrix and explicitly mark unsupported combinations.
- Pin the Rust toolchain used by local development and CI.
- Add workspace lint configuration and make every crate inherit it.
- Record baseline data for build time, binary size, test count, startup, idle CPU, frame time, and memory.
- Correct README claims about accessibility status, runtime dependencies, and available examples.
- Inventory public items and all `unwrap`, `expect`, `panic`, `todo`, and `unsafe` sites in library code.

### Required CI jobs

```text
format
default clippy, all targets
default tests
all-features check
individual checks for a11y, mesh3d, portals, settings, sandbox, markup
release build
documentation build with warnings denied
```

Use `cargo hack` or an equivalent scripted matrix if it materially reduces duplication.

### Exit criteria: Gate 0

- The supported matrix is green on a clean Linux CI runner.
- Baseline results are stored in a versioned document or benchmark output.
- Unsupported behavior is not advertised as available.
- No architectural implementation begins with an unexplained red build.

## 8. Phase 0.5: Decision Spikes

**Goal:** Resolve the adopt-versus-build questions that determine the shape of every later phase, before any restructuring begins.

These spikes were previously listed as a parallel research series, but their outcomes gate the ADRs: running them late risks invalidating finished work. Each spike is time-boxed to one week and ends with a short written result — adopted, rejected with reasons, or escalated to a focused decision. Spikes must not become indefinite parallel implementations.

### Spikes

1. **Taffy comparison** against current flex, grid, scroll, and intrinsic-text cases. Gates `ADR-001-frame-lifecycle.md`: the outcome decides whether Phase 1 is an integration exercise or an in-house solver redesign.
2. **Current-frame display-list or element-tree prototype** — the Phase 1 candidate designs, in miniature. Also gates ADR-001.
3. **AccessKit prototype** with a small subset of the Phase 4 representative widget set. Gates `ADR-002-core-renderer-boundary.md`: the semantic types introduced in Phase 2 must match the chosen source of truth.
4. **Parley/cosmic-text Unicode editing comparison** against the rustybuzz/swash stack. Gates the text-layout interface in Phase 2's dependency diagram.
5. **Lavapipe WGPU CI proof of concept.** Not blocking; informs Phase 6 feasibility and may run in parallel with anything.

### Exit criteria: Gate 0.5

- Spikes 1–4 have recorded outcomes, referenced by the ADRs they gate.
- Review questions 1 (immediate-mode requirement) and 7 (AccessKit as source of truth) from §18 have maintainer answers recorded — every downstream decision bends around these two.

### Recorded Gate 0.5 outcome

Gate 0.5 is complete. The four blocking spike records are:

- `docs/GATE_0_5_TAFFY_SPIKE.md`;
- `docs/GATE_0_5_CURRENT_FRAME_SPIKE.md`;
- `docs/GATE_0_5_ACCESSKIT_SPIKE.md`; and
- `docs/GATE_0_5_TEXT_STACK_SPIKE.md`.

Question 1 is answered by the current-frame and lifecycle decisions:
immediate-mode syntax is the application-facing API, while the internal frame
uses one owned current-frame tree built by one application declaration.
Question 7 is answered by the accessibility spike: AccessKit is an adapter over
an Esox-owned serializable semantic tree. The consolidated evidence and
remaining non-blocking question are in `docs/GATE_0_5_STATUS.md`.

## 9. Phase 1: Define the Frame Contract

**Goal:** Eliminate previous-frame layout as the source of current-frame painting.

### Decision to make

Choose how immediate-mode declarations become a current-frame resolved scene. The Gate 0.5 Taffy comparison and current-frame prototype are required inputs to this decision. Candidate designs include:

1. Record widget and paint commands, solve layout, then resolve commands into a display list.
2. Build a lightweight element tree, solve it, then run a dedicated paint traversal.
3. Retain a reconciled tree behind the immediate-mode facade.

Replaying application closures for a second pass is not acceptable unless side-effect behavior is formally constrained and tested.

### Required ADR

`ADR-001-frame-lifecycle.md` must specify:

- Frame phases and allowed mutation in each phase.
- When widget responses become observable.
- Whether input is dispatched against the last committed tree or the tree being built.
- How newly inserted and removed widgets behave during input dispatch.
- How intrinsic measurement accesses text and images.
- How overlays participate in layout, focus, hit testing, clipping, and z-order.
- What causes layout, paint, and semantic invalidation.
- How animations request subsequent frames.
- Whether the frame-1 cursor fallback accepted in the 2026-03-26 tree-primary layout decision survives. Gate 1 as written supersedes that decision; the ADR must record the reversal and its rationale explicitly.

### Contract tests

These tests run in CI from day one, which requires a minimal headless harness — deterministic text measurement and a null renderer — pulled forward from Phase 2. Full headless construction still lands in Phase 2; Gate 1 must not depend on a windowed environment.

- The first rendered frame equals the second frame when inputs are unchanged.
- A viewport resize produces correct geometry in the first post-resize frame.
- Adding, removing, hiding, and reordering children does not paint stale positions.
- Text or theme metric changes update layout and paint in one frame.
- Paint rect, hit rect, clip rect, accessibility rect, and damage rect derive from one resolved node.
- Scroll offset changes do not require stale-layout exceptions.
- Overlays appear above content and cannot be hit through their blocking region.

### Exit criteria: Gate 1

- No user-visible primitive consumes `prev_layout` for current-frame painting.
- Frame contract tests pass without rendering two warm-up frames.
- Resize-specific cache invalidation is an optimization, not a correctness mechanism.
- ADR-001 documents how the design supports headless rendering and multiple windows, with a test or prototype demonstrating each claim.

## 10. Phase 2: Establish Core Boundaries

**Goal:** Make layout, interaction, and semantics independent from WGPU and window creation.

### Target dependency direction

```text
esox_input/core types
        |
        v
esox_core  <--- text layout interface
   |   |
   |   +------> semantic tree
   v
display list
   |
   v
esox_renderer_wgpu
   |
   v
esox_platform_winit

Optional: esox_3d integrates with the renderer, not esox_core.
```

Crate names are provisional; dependency direction is the requirement.

### Deliverables

- Introduce GPU-independent geometry, layout, display-list, and semantic types.
- Make the display list and semantic tree serializable and inspectable as structured data. This serves snapshot testing now and is the substrate for the declarative description layer (§6) later; design with that consumer named.
- Move window/surface creation out of the general graphics API.
- Separate font discovery, shaping, rasterization, cache policy, and GPU upload.
- Make headless UI construction possible with deterministic font fixtures.
- Define renderer capabilities and graceful fallback behavior.
- Move optional 3D rendering into a separate crate or an equally isolated boundary.
- Define per-window ownership of UI state, render state, scale factor, focus, cursor, and accessibility adapter.

### Required ADRs

- `ADR-002-core-renderer-boundary.md`
- `ADR-003-resource-and-device-lifetime.md`
- `ADR-004-multi-window-state-ownership.md`

### Exit criteria: Gate 2

- Layout and semantic tests run without initializing WGPU or Winit.
- A display list can be inspected and snapshot-tested as structured data.
- Renderer loss or surface recreation does not destroy application/UI state.
- Two independent window contexts can exist in a test harness.
- All preceding criteria are demonstrably green; carefully selected widget work may then resume without further ceremony.

## 11. Phase 3: Coordinates, Identity, and Focus

**Goal:** Establish the shared primitives on which every widget depends.

### Coordinates

- Add distinct logical and physical size, point, and rectangle types or typed wrappers.
- Define rounding rules at rasterization and hit-test boundaries.
- Specify transform composition, inverse hit testing, clipping, and accessibility bounds.
- Test fractional scale factors such as 1.25, 1.5, and 1.75.

### Identity

- Add hierarchical ID scopes for reusable components and repeated collections.
- Detect duplicate live IDs in debug builds.
- Specify identity behavior for keyed and unkeyed children during reorder.
- Separate widget identity from animation, layout-node, semantic-node, and render-resource identities.
- Define state retention and garbage-collection policy for widgets that disappear.

### Focus

- Represent focus scopes explicitly.
- Trap focus within modal dialogs and restore focus to the invoker on close.
- Support initial focus, disabled/hidden removal, directional navigation, and programmatic focus.
- Keep keyboard focus separate from pointer capture and accessibility focus where required.

### Exit criteria: Gate 3

- DPI tests prove consistent paint and input behavior across scale factors.
- Reusable components do not require globally unique string literals for descendants.
- Modal, popover, menu, tab, tree, table, and form focus tests pass.
- Focus order is inspectable in the debug tooling or test API.
- The `0.x` experimental release described in §6 ships.

## 12. Phase 4: Accessibility Architecture

**Goal:** Make accessibility an enforced framework property rather than an optional metadata stream.

### Building on the Gate 0.5 spike

The AccessKit-versus-direct-AT-SPI decision was made at Gate 0.5. This phase extends the winning prototype to the full representative set:

- Button and toggle actions.
- Text input with cursor, selection, insertion, and value updates.
- Slider value interface.
- Table or tree hierarchy and selection.
- Modal focus and window events.
- Scrolled and clipped bounds.

Direct AT-SPI remains the fallback only if a documented capability gap cannot be resolved upstream.

### Semantic model requirements

- Hierarchical parent/child relationships.
- Roles, names, descriptions, values, states, and relations.
- Action dispatch back into the same widget interaction system.
- Text ranges, cursor, selection, and editable-text operations.
- Live regions and announcements.
- Incremental updates with stable IDs and event diffs.
- Coordinate conversion to screen/window space.
- Virtualized collection semantics without materializing every visual child.

### Validation

- Automated semantic-tree snapshots.
- Action round-trip tests.
- AT-SPI adapter integration tests where feasible.
- Manual Orca test scripts for every interactive widget family.
- Keyboard-only completion of the demo's primary workflows.

### Exit criteria: Gate 4

- Orca can discover, navigate, read, and activate the representative widgets.
- Semantic hierarchy matches visual hierarchy except where intentionally documented.
- Modal background content is inert to keyboard, pointer, and accessibility actions.
- Accessibility is enabled by normal runtime integration rather than a nonfunctional feature flag.

## 13. Phase 5: Input and Text Correctness

**Goal:** Support real Linux desktop input and international text without widget-local workarounds.

### Unified input model

- Pointer ID, device kind, buttons, position, pressure where available, and event phase.
- Explicit pointer capture and release/cancellation behavior.
- Horizontal and vertical scrolling in pixel and line units without arbitrary conversion loss.
- Touchscreen events and multi-touch policy.
- Double-click/click-count and drag threshold based on platform settings where available.
- Window focus loss, suspension, device loss, and cancellation cleanup.
- Synthetic and accessibility actions routed through the same command path.

### Text stack validation

The Parley/cosmic-text/rustybuzz direction was chosen at Gate 0.5. Validate the chosen stack against:

- UAX #9 bidirectional text.
- UAX #14 line-breaking opportunities.
- UAX #29 grapheme and word segmentation.
- Mixed-script font fallback and emoji sequences.
- Ligatures and cursor positions within shaped clusters.
- IME preedit, candidate-window placement, commit, and cancellation.
- Selection movement in visual and logical directions.

Adopt proven components where they meet requirements. Keep custom GPU glyph upload only where it remains a real differentiator.

### Exit criteria: Gate 5

- Unicode conformance data covers segmentation and line breaking.
- Combining marks and emoji sequences are edited as grapheme clusters.
- Mixed RTL/LTR paragraphs render and edit correctly.
- IME candidate placement follows the active caret.
- Pixel scrolling remains pixel-precise and horizontal scrolling is preserved.
- Pointer capture behaves correctly when leaving the widget or window.

## 14. Phase 6: Regression and Performance Infrastructure

**Goal:** Test the framework behavior users actually see.

### Test pyramid

1. Pure unit tests for geometry, layout, damage, IDs, parsers, and algorithms.
2. Headless interaction tests for widgets, focus, semantics, and input replay.
3. Structured display-list snapshots for deterministic rendering intent.
4. WGPU tests using Lavapipe or another CI-capable Vulkan software adapter.
5. Golden images for a curated, stable visual suite.
6. Manual release checks with real hardware, Orca, IME, fractional DPI, and multiple compositors.

Wayland and X11 smoke tests in virtual sessions are deferred: add them only after items 1–5 are established. Until then that coverage lives in the manual release checks.

### Golden-image policy

- Snapshot only after Gate 1 establishes correct current-frame layout.
- Store renderer, font fixture, scale factor, color space, and tolerance metadata.
- Prefer small focused scenes over a single monolithic gallery screenshot.
- Require human review for baseline updates.
- Start with a small set of focused scenes at 1.0 and 1.5 scale in light and dark themes. Add high-contrast and viewport variants per-scene when a regression justifies them — a full matrix is a baseline-maintenance burden a solo maintainer cannot service.
- Use semantic and display-list snapshots to diagnose pixel differences.

### Performance budgets

Define budgets for:

- Cold startup and first useful frame.
- Idle CPU and GPU activity.
- UI build, layout, paint preparation, upload, and submission time.
- 10k-item virtual list and table scrolling.
- Text shaping and cache miss behavior.
- Damage-only updates versus full redraw.
- Resize latency and animation frame pacing.
- Memory growth during repeated insertion/removal and long sessions.

Performance regressions should report distributions and environment metadata, not rely on a single noisy threshold.

### Exit criteria: Gate 6

- Representative widgets are protected by interaction and semantic tests.
- The WGPU render path executes in CI.
- Golden tests identify meaningful visual regressions with a documented update process.
- Benchmarks are reproducible and tracked over time.
- Idle windows do not redraw continuously without an explicit reason.

## 15. Phase 7: Public API and Release Readiness

**Goal:** Make Esox safe for external experimentation without freezing implementation details.

### API work

- Define a supported facade and prelude for each published crate.
- Make internal modules private by default.
- Categorize public types as stable, experimental, or internal.
- Add `#[non_exhaustive]` only where forward-compatible external matching or construction is intended.
- Document error, panic, cancellation, threading, coordinate, and lifecycle behavior.
- Audit `unwrap`, `expect`, and `panic` sites; convert recoverable failures to typed errors.
- Enable `missing_docs` enforcement on the intended public surface.
- Add rustdoc examples that compile in CI instead of broadly ignored examples.
- Add API-diff checking such as `cargo-semver-checks`.

### Project work

- Pin and document an MSRV policy.
- Add changelog, security policy, contribution architecture notes, and release checklist.
- Add package metadata and verify packaged crate contents.
- Run dependency advisory, license, and source-policy checks.
- Maintain at least one downstream canary application outside the workspace.

### Release criteria: Gate 7

- A new user can build a representative application from public documentation alone.
- The canary application uses no internal modules.
- All supported configurations, documentation, snapshots, and smoke tests are green.
- Accessibility and international text claims have corresponding validation evidence.
- Known limitations are explicit and do not contradict the README.
- The semver and deprecation policy is written down and applied to the release.

## 16. Proposed Review Artifacts

Each phase should produce evidence, not only code:

| Artifact | Purpose |
| --- | --- |
| ADRs | Record durable decisions and rejected alternatives |
| Contract tests | Turn architectural invariants into executable checks |
| Feature matrix | Define what the project actually supports |
| Baseline report | Quantify performance, size, and build behavior before changes |
| Semantic snapshots | Review accessibility independently of pixels |
| Display-list snapshots | Diagnose rendering intent without GPU variability |
| Golden images | Protect final raster output |
| Manual test scripts | Cover Orca, IME, compositors, and real GPU behavior |
| Downstream canary | Validate API usability and packaging |

This is a solo-maintained project, so "review" means something concrete: a gate is approved when its contract tests pass and a written self-review checklist for its ADRs and exit criteria is committed alongside the evidence. External review is invited where available but is not what a gate waits on.

## 17. Initial Backlog

### First pull request series

1. Restore clippy and `a11y` feature compilation.
2. Add workspace lint policy and feature-matrix CI.
3. Correct README and roadmap status claims.
4. Add first-frame and resize contract tests that expose the current lifecycle behavior.
5. Write `ADR-001-frame-lifecycle.md` once the Gate 0.5 spike results are recorded.

The research series that previously sat alongside this list is now Phase 0.5 (§8) and runs immediately after — or interleaved with — the first three items above.

## 18. Review Questions

Reviewers should resolve these questions before approving the plan. Questions 1 and 7 are blocking: they gate Phases 1 and 2 respectively and must be answered at Gate 0.5. The rest must be resolved before their affected phase begins.

1. Is immediate-mode syntax a requirement, or only the desired application-facing API?
2. When should widget responses be available relative to current-frame layout?
3. Is a CPU/software renderer a supported fallback or only a testing tool?
4. Which feature combinations are part of the support contract?
5. Is 3D rendering a core product requirement or a separate integration?
6. Is Linux-only a durable scope decision, and which Wayland/X11 environments are supported?
7. Will AccessKit be the semantic source of truth, or an adapter over an Esox-owned semantic model?
8. What level of Unicode and locale support is required for the first public release?
9. Which widgets define the minimum representative release set?
10. What stability promise, if any, accompanies the first crates.io release?

## 19. Definition of Done

This stabilization program is complete when every gate — Gate 0 through Gate 7, including Gate 0.5 — is approved and its evidence is available in the repository. Completion is not measured by widget count. It is measured by whether applications can trust frame correctness, input, text, accessibility, resource lifetimes, public API behavior, and supported Linux configurations.
