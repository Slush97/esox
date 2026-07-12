# Gate 1 production FrameCore integration plan

**Status:** In progress; the FrameCore production declaration path is live, but
Gate 1 remains open until the production `Ui` no longer reads previous-frame
layout.

## Integration progress (2026-07-11)

Completed and covered by production-API or renderer-boundary tests:

- backend-neutral scene, semantic, paint, measurement, and submission records;
- the once-only production declaration vertical slice for flex/grid containers,
  text, buttons, solid rectangles, and borders;
- renderer-neutral production image declarations with GPU-independent intrinsic
  measurement, semantic labels, and transactional pointer responses;
- renderer-neutral horizontal and vertical separator declarations with
  authoritative cross-axis thickness, exact solid paint, semantic output, and
  typed invalid-thickness rejection;
- current-frame constraints, alignment, hidden/disabled participation, clipping,
  retained scrolling, transforms, damage expansion, and blocking overlays;
- transactional pointer and wheel dispatch, focus/capture reconciliation, and
  logical-to-physical conversion at platform and renderer boundaries;
- a renderer-neutral transactional keyboard ledger routed to committed focus,
  with chronological delivery, deterministic retry, and per-window isolation;
- detached split-phase generation attempts that preserve `Ui::begin`/`finish`
  ownership ergonomics while retaining atomic commit, retry, and per-window
  isolation;
- production split panes with current-frame ratio layout, transactional drag
  capture, committed cursor metadata, and same-batch ordered pointer routing;
- renderer-neutral uniform virtual content with candidate-generation retained
  and wheel state, full logical content extents, once-only visible item
  declaration, stable logical item identities, current-generation scroll-to and
  clamping, and transactional retry;
- a renderer-neutral fixed-track production table declaration built on one
  virtual column, with once-only visible row callbacks, logical row/cell IDs,
  chronological external sort and selection intents, transactional column
  resizing and input replay, committed-row selection across same-batch scroll,
  shared current-generation header/body tracks, and retry-safe keyboard row
  navigation through Arrow, Home, End, Page, Enter, and Space keys; and
- headless first-frame, resize, structural, metric, scroll, transform, overlay,
  damage, and multi-owner contract coverage.

Still required before Gate 1 closes:

- migrate compound containers and the remaining production leaves;
- migrate the legacy `Ui::virtual_scroll` and table caller surfaces onto the
  FrameCore virtual-content and table declarations;
- route the existing application-facing `Ui::begin`/`Ui::finish` path through
  one `FrameCore` owner per window;
- remove production `prev_layout`, `layout_cache`, cursor fallback, and
  closure-based measurement dependencies; and
- run the complete cutover validation matrix and record Gate 1 status.

## Required outcome

Production frames must execute the ADR-001 lifecycle:

1. dispatch input against the last committed scene;
2. invoke the application declaration exactly once into an Esox-owned
   current-frame element tree;
3. measure GPU-independent leaves and solve that tree with Taffy;
4. traverse the resolved tree once to derive paint, effective clips, hit
   records, semantic records, focus order, and current damage;
5. reconcile interaction and atomically commit the generation; and
6. submit the committed display list to `Frame`.

The public immediate-mode syntax may remain. Its widget calls become
declarations; they may not depend on solved geometry while the application
closure is running.

Gate 1 is complete only when all of the following are true:

- `Ui` has no `prev_layout` field or previous-frame geometry lookup;
- `UiState` has no `layout_cache` used as a current-frame geometry source;
- first-frame, resize, structural, metric, scroll, and overlay output comes
  from the generation being committed;
- the production path runs the application closure once;
- production text and image measurement can run without WGPU, a surface, or a
  glyph atlas; and
- the twelve headless FrameCore contracts still pass through the shared
  lifecycle and geometry model.

## Why the integration cannot be a finish-time translation

The current `Ui` does more than paint at the rectangle returned by
`allocate_rect`. During declaration it also uses that rectangle to choose text
wrapping and truncation, establish GPU and hit clips, register hit/focus data,
emit accessibility bounds, size nested regions, and sometimes position later
siblings. Solving at `finish()` and translating already-emitted instances
would correct only a subset of those products.

Likewise, running the application closure once to measure and again to paint
would violate ADR-001. `Ui::measure` currently embodies that pattern for a
subtree, although it has no in-tree callers; it must not be part of the new
pipeline.

The narrow safe integration is therefore a staged internal migration with an
atomic whole-frame cutover. A legacy and a current-frame implementation may
coexist while the latter is tested, but a committed scene may never combine
geometry or derived records from both.

## Ownership boundary

The production implementation should keep four kinds of data separate:

| Owner | Lifetime | Contents |
| --- | --- | --- |
| Per-window state | Across frames | Input ledger, focus/capture, scroll offsets, animation values, widget state, resources, last committed scene |
| Element tree | One generation | Stable IDs, hierarchy, Taffy style inputs, intrinsic measurement requests, paint properties, interaction properties, semantic properties |
| Resolved scene | One committed generation | Unrounded logical bounds, effective clips, paint list, hit index, Esox semantic snapshot, focus order/scopes, damage |
| Renderer | Submission/resource lifetime | Logical-to-physical conversion, snapping, WGPU instances, glyph rasterization and atlases |

`FrameCore` should own the first three data domains. The present
headless `Element` and `CommittedScene` are deliberately small contract
fixtures; production work should generalize their data model rather than put a
second lifecycle beside them.

FrameCore-owned mutable state is transactional across dispatch, declaration,
resolution, and interaction reconciliation. Each attempted generation uses a
candidate `WidgetStateStore`, scroll map, focus/capture state, focus scopes,
restoration map, and cancellation queue. Pointer, wheel, and keyboard queues
are cleared only with a successful candidate commit; a rejected attempt leaves
the queues and all persistent state unchanged for deterministic retry against
the same committed scene. Keyboard events contain only `esox_input` values and
route in queue order to the focus of that immutable committed scene; focus
requests made during declaration cannot retarget them. Unconsumed keyboard
responses are discarded when their target leaves the active focus order.
Application-owned side effects performed by declaration are not rollbackable
and remain outside this boundary.

Wheel input is dispatched against the last immutable committed scene. Routing
starts at the topmost visible, enabled structural node under the wheel position
and follows only that node's committed parent chain, so overlapping siblings
behind a blocking overlay are ineligible. X and Y route independently to the
deepest scroll viewport on that chain that can change on the corresponding
axis. A viewport that changes consumes that axis for the whole event, including
when it reaches an edge after applying only part of the delta; residual delta is
not propagated to ancestors. A still-declared hidden or disabled viewport keeps
its stable-ID offset, but neither participates in wheel routing. Removing the
viewport drops its retained state. Explicit declaration offsets override
retained input state whenever the viewport participates in that generation.
Raw same-direction wheel intent is retained separately from the old committed
clamp so repeated deltas can apply if that viewport's content grows in the
generation being declared. This does not retroactively change committed-scene
routing: when a committed ancestor can scroll, it consumes the axis and later
inner growth does not reroute that event to the descendant.

A pointer response dispatched to a committed virtual descendant is retained for
one successful generation when same-batch scrolling moves that item outside the
new visible range and the virtual owner remains active. It is available if the
item is declared again in the immediately following generation, then expires;
it is not generic stale response retention. Pointer capture is stricter: a
captured virtual descendant leaving the declared range loses capture and emits
the normal cancellation even while its virtual owner remains present.

FrameCore reserves deterministic wrapper IDs derived from the virtual viewport
and logical item index. Application declarations own their descendant IDs and
must not reuse a wrapper ID; collisions fail duplicate-ID validation atomically.

The platform boundary owns cursor validity and redraw eligibility per window.
Cursor state has no coordinate sentinel: it is unavailable until a finite
position arrives, and becomes unavailable again on leave, focus loss,
suspension, or destruction. Position-dependent pointer and wheel events are
rejected while unavailable rather than being routed at `(0, 0)`. Focus gain,
resume, and position-less entry do not revive a stale coordinate.

Headless and production redraw routing qualify a raw `WindowId` with a window
incarnation and one coalesced pending redraw serial. Only a live matching
incarnation with that pending serial accepts delivery, and acceptance consumes
the serial before frame execution. Unknown, destroyed, suspended, stale, and
duplicate deliveries are inert; they never fall back to another live window.
Suspension or destruction clears pending redraw eligibility. This boundary
prevents rejected delivery from duplicating frame execution, submission, input
consumption, or cancellation while leaving failed FrameCore generations
retryable under the transactional contract above.

Coordinate conversion is asymmetric and single-owner. Winit physical cursor
positions are divided once by the named window incarnation's validated
event-time scale. Pointer and wheel capture use that same logical position.
Physical-pixel wheel deltas are divided by the event-time scale and the fixed
logical-units-per-line normalization constant; line-wheel deltas preserve both
already-normalized axes. Native direction is inverted once on both axes before
FrameCore. Queued events contain logical values only, so later cursor, scale,
viewport, or other-window changes cannot reinterpret them.

Resize and scale-factor events replace the window-local transform before the
logical viewport callback and next redraw. FrameCore accepts only finite,
positive logical viewports and stores unrounded logical committed geometry.
Renderer submission owns the reverse transform and multiplies that geometry by
one validated scale exactly once; WGPU clip quantization remains the snapping
stage. Invalid positions, deltas, scales, viewports, or transform overflow are
rejected before persistent or renderer mutation and have no fallback target.

The semantic record remains an Esox type. A later AccessKit adapter consumes a
committed semantic snapshot and does not own widget hierarchy or bounds.
The current virtualization slice exposes a `ScrollView` semantic node and only
the semantic rows in the visible declared range. Virtual collection size/index
metadata, offscreen accessibility navigation, and accessibility scroll actions
remain required before accessibility support for virtual collections can be
called complete. The production table slice likewise exposes only currently
declared interactive rows and generic header interactions: table/header/cell
roles, row/column metadata, sort state, and offscreen row navigation are not yet
represented and must not be advertised as complete table accessibility.

## Production-neutral leaf boundaries

Layout leaves carry measurement input, not renderer state:

```text
TextMeasureRequest {
    content,
    font properties,
    locale hint,
    direction hint,
    known dimensions,
    available space,
}

ImageMeasureRequest {
    resource key,
    decoded intrinsic dimensions or placeholder dimensions,
    known dimensions,
    available space,
}
```

The Phase 1 adapter may initially wrap existing deterministic or CPU font
metrics. Its interface must accommodate the selected cosmic-text 0.19 backend
without exposing cosmic-text types. Shaping and measurement cannot upload
glyphs or consult an atlas. Rasterization happens only when the resolved paint
list is submitted.

IME composition remains in Esox per-window/widget state. The element tree
contains the composition presentation declared for that generation, not the
mutable composition owner.

## Implementation slices

### 1. Share the lifecycle and scene vocabulary

- Split the contract-only conveniences from reusable FrameCore types: logical
  geometry, stable IDs, input ledger, element hierarchy, resolved nodes,
  semantic nodes, and atomic commit.
- Add production paint and semantic properties as Esox-owned data attached to
  element nodes. Do not store application callbacks in nodes.
- Make scene consumption a renderer-neutral trait. Keep
  `NullSceneConsumer`; add the WGPU/`Frame` consumer only at the renderer
  boundary.
- Preserve one `FrameCore` per `UiState`/window. Do not put committed geometry
  in global caches.

This slice stays headless and should extend the existing contract tests rather
than create a second test harness.

### 2. Add the production declaration vertical slice

Implement declarations for the smallest representative set:

- row and column containers;
- padding, gap, fixed/min/max constraints, and flex grow;
- label/paragraph text leaves;
- button interaction and semantics; and
- solid rectangle, border, and text paint primitives.

Run this slice through Taffy and the resolved traversal in a headless
production-API test. The test must declare once and verify first-frame and
first-post-resize paint, hit, clip, semantic, and damage bounds.

The application-facing calls should retain their current shape where possible.
Responses come from the event ledger populated from the last committed scene,
not from hit testing the tree under construction.

### 3. Migrate geometry-sensitive containers

Move containers in dependency order:

1. constrained/max-width/centered regions and flex/grid;
2. hidden and disabled state;
3. clipping and scroll containers using current scroll offsets during resolve;
4. transforms and damage expansion;
5. overlays, blocking regions, focus scopes, and portal ownership; and
6. tables, split panes, virtual scrolling, and other compound widgets.

Container declarations record relationships and style. They do not save and
restore solved cursor rectangles. Overlay anchoring is resolved from the
anchor node in the same generation.

### 4. Migrate remaining leaves and renderer submission

- Represent every existing shape/image/text operation as an owned paint
  primitive or a renderer-neutral resource reference.
- Resolve text wrapping, truncation, alignment, and cursor geometry after
  Taffy supplies constraints.
- Build hit, focus, accessibility, clip, and damage records in the same
  resolved traversal that creates paint records.
- Convert the resolved paint list into `Frame` instances afterward. Pixel
  snapping and atlas access remain in this consumer.

### 5. Cut production `Ui` over atomically

- Route `Ui::begin` through per-window FrameCore dispatch and declaration
  setup.
- Make `Ui::finish` perform measure/layout, resolve, reconcile, commit, and
  renderer submission in order.
- Remove `prev_layout`, `lookup_solved`, `cursor_fallback`, and the production
  `layout_cache` dependency.
- Remove or replace `Ui::measure`; intrinsic measurement must use leaf
  requests, never execute widget/application closures.
- Retire the in-house solver from the production path. Keep it temporarily
  only if isolated tests or an explicitly non-production compatibility path
  still require it, then remove it separately.

Do not retain a fallback that selects legacy geometry when a new declaration
kind is missing. An unsupported migrated widget must fail a test/build-time
inventory check rather than silently create a mixed-generation scene.

## Atomic change sequence

The expected commit sequence is:

1. add reusable owned scene/semantic/paint records and tests;
2. add the GPU-independent production measurement interfaces;
3. add the basic production declaration vertical slice and headless tests;
4. add the renderer consumer for resolved paint records;
5. migrate container groups in independently tested commits;
6. migrate leaf/compound widget groups in independently tested commits;
7. switch `Ui` to FrameCore and delete previous-frame geometry; and
8. remove obsolete layout/cache code and update Gate 1 status.

Each commit should compile and test independently. Existing unrelated and
untracked files are outside these commits.

## Validation

Every slice runs:

```sh
cargo fmt --all -- --check
cargo test -p esox_ui --all-features
cargo clippy -p esox_ui --all-features --all-targets -- -D warnings
git diff --check
```

Before the cutover commit, also run:

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --manifest-path spikes/accesskit_compare/Cargo.toml
cargo clippy --manifest-path spikes/accesskit_compare/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path spikes/text_stack_compare/Cargo.toml
cargo clippy --manifest-path spikes/text_stack_compare/Cargo.toml --all-targets -- -D warnings
```

Add a production-API regression for each Gate 1 contract as the relevant
widget group migrates. The final cutover requires a source check showing no
production `prev_layout`, cursor fallback, or application-closure measurement
path remains.

## First implementation task

The next atomic code change should generalize the owned scene records without
changing production `Ui` behavior:

- define backend-neutral paint primitives and semantic properties on current
  elements;
- have the resolved traversal derive paint, hit, clip, semantic, and damage
  records from one resolved node;
- keep `NullSceneConsumer` snapshots deterministic; and
- prove the new types remain GPU/platform independent.

That creates the seam needed by the production declaration vertical slice
without broadening the current uncommitted FrameCore contract work into a
premature `Ui` rewrite.
