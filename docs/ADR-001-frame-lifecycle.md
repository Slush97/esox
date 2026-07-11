# ADR-001: Frame lifecycle

**Status:** Accepted; implementation pending
**Recorded:** 2026-07-11
**Decision owners:** Esox maintainers
**Supersedes:** The frame-1 cursor fallback accepted by the 2026-03-26
tree-primary layout decision

## Context

Esox presents an immediate-mode widget API, but its production implementation
currently paints while declarations execute, solves the layout tree at the end
of the frame, and uses that solved tree during the next frame. That makes the
first frame a cursor-positioned approximation and lets structural, metric,
scroll, and viewport changes paint from stale geometry.

Two Gate 0.5 spikes determine the replacement:

- [the Taffy comparison](GATE_0_5_TAFFY_SPIKE.md) adopts Taffy for flex, grid,
  constraint propagation, and intrinsic measurement queries; and
- [the current-frame comparison](GATE_0_5_CURRENT_FRAME_SPIKE.md) adopts an
  ephemeral lightweight element tree followed by layout and a dedicated
  resolved traversal.

The latter rejects a recorded command stream because it must reconstruct a
tree before layout, and rejects a reconciled render tree as a Phase 1
correctness requirement. Neither application-closure replay nor previous-frame
geometry is an acceptable way to obtain current-frame results.

## Decision

Each window constructs and commits one resolved scene generation at a time:

```text
platform events
      |
      v
dispatch against last committed scene
      |
      v
declare once -> ephemeral element tree
      |
      v
measure and lay out with Taffy
      |
      v
resolved traversal
  |       |       |       |       |
paint    clips   hits   semantics damage
      \     |       |       |     /
             atomic scene commit
                     |
                     v
             renderer / null sink
```

Application declarations execute exactly once for a scene generation. The
ephemeral tree is dropped after resolution and commit. Stable widget identity,
interaction state, scroll offsets, animation state, focus, capture, and
resource caches live in per-window stores outside that tree.

Taffy receives the complete current-frame hierarchy and returns unrounded
logical geometry. A dedicated traversal of that geometry produces the display
list, effective clips, hit-test index, semantic snapshot, and current damage
bounds. Those products cannot perform layout independently or substitute
geometry from an older generation. Logical-to-physical conversion and pixel
snapping occur only at the renderer boundary.

The last committed scene remains useful for routing input and comparing old
and new damage. It is historical state, not a current-frame correctness source.

## Frame phases and mutation

The phases below are ordered for each window. Work for different windows may
interleave, but a window has at most one scene generation under construction.

| Phase | Work | Allowed mutation |
| --- | --- | --- |
| 0. Collect | Queue platform, timer, resource, and accessibility events; request a frame. | Pending queues and scheduling state only. |
| 1. Dispatch | Route queued input through the last committed hit/semantic tree or an existing capture path. Produce an event-response ledger for stable widget IDs. | Input state, focus/capture requests, and the ledger. The committed scene is immutable. |
| 2. Declare | Invoke the application once and build the owned current-frame element tree, including overlay declarations. | Application state, per-widget persistent state, and the new tree. No solved-geometry query is available. |
| 3. Measure/layout | Adapt the tree to Taffy, run constrained leaf measurements, and solve logical geometry. | Taffy-local caches and resolved geometry only. Application and widget declarations are immutable. |
| 4. Resolve | Traverse resolved nodes in paint order to create display-list, clip, hit, semantic, and damage records. | Generation-local output builders only. |
| 5. Reconcile interaction | Validate focus, capture, hover, and overlay ownership against the resolved live nodes; synthesize required cancellation and restoration. | Per-window interaction state and next-frame scheduling. Resolved geometry is immutable. |
| 6. Commit | Atomically replace all products of the previous committed generation. Hand the display list and damage to a renderer or null sink. | The per-window committed-scene pointer and renderer submission state. |
| 7. Schedule | Present when applicable and request the next deadline for active animation, pending work, or recovery. | Platform scheduling and diagnostics only. |

If construction or resolution fails, none of its partial products become
committed. The previous scene remains intact for diagnostics and input until a
later generation commits or the window closes. Renderer or surface failure
does not destroy UI/application state; it schedules recovery through the same
scene contract.

### Measurement

Text and image leaves expose GPU-independent measurement inputs. Taffy may ask
a leaf to measure more than once using different known dimensions or available
space. A measurement callback therefore must be deterministic for its inputs,
must not mutate application-visible state, must not emit paint, and must not
invoke application code.

Text measurement uses shaped font metrics independent of a glyph atlas. Image
measurement uses decoded metadata or an explicit placeholder size. Arrival of
font, image, or theme metrics requests a new frame; the resulting layout and
paint change together in that generation.

## Input dispatch and response timing

Input targets the last committed scene, never the tree under construction.
This avoids dispatch order depending on declaration order and gives events a
complete, visible hit-test and semantic tree. Pointer hit testing examines the
committed scene in reverse paint order after transforms and effective clips.
Keyboard events target keyboard focus. Accessibility actions use the same
routing contract. An active pointer capture overrides hit testing for its
pointer until release or cancellation.

Each routed event is stamped with the committed scene generation and stable
target/path IDs. Dispatch updates framework interaction state and records the
result in a per-window ledger. A widget observes its pending `Response` when it
is declared during the next frame. Response flags are consumed at most once;
hovered, pressed, and focused state reflects the dispatch result for that
frame. Any event coordinates or target bounds exposed by a future response API
refer to the committed generation that received the event, not unsolved
current-frame geometry.

Application code may mutate its own state after observing a response, and that
mutation affects declarations that have not yet executed. It does not rewrite
elements already declared earlier in the same closure. If an earlier element
must change as a result, the mutation requests another frame. Esox never
replays the closure to make that change appear sooner.

Multiple events retain routing order in the ledger even when today's boolean
`Response` surface coalesces them. Phase 1 may preserve the current response
shape, but its internals must not make event ordering impossible to expose
later.

## Inserted, removed, hidden, and reordered widgets

Widget liveness is determined solely by the current declaration and resolved
tree:

- An inserted widget participates in layout, paint, clips, semantics, damage,
  and the committed hit index immediately. It cannot receive an event routed
  before its first commit. Programmatic initial-focus requests are resolved at
  commit.
- A widget present in the dispatch generation may receive a pending response
  even if application state later removes it. If it is not declared, the
  response is discarded at reconciliation. The current scene contains no
  geometry or output for it.
- A widget removed or hidden by the current declaration loses hover and
  keyboard focus at reconciliation. Any pointer capture it owns receives a
  cancellation and is released before commit. Overlay focus restoration then
  applies if relevant.
- `hidden` means absent from layout, paint, hit testing, focus order, and
  semantics for that generation. A future layout-preserving visibility mode
  must be a distinct state with an explicit semantic and interaction contract.
- Reordering changes current paint order, focus order, semantic order, and hit
  precedence in the same generation. Persistent state follows stable widget
  identity, not sibling position. Duplicate live IDs are an error to detect in
  debug/test builds.

Persistent widget state may survive absence according to a bounded garbage
collection policy defined with the identity work in Phase 3. Retention never
retains the widget's resolved geometry or makes the ephemeral render tree
persistent.

## Overlays, focus, and capture

Overlays are declared into explicit per-window layers in the current-frame
tree. A portal may retain semantic ownership by its declaration site while its
layout and paint node is attached to an overlay layer. Anchor placement uses
the anchor's current resolved geometry in the same layout generation.

Base content resolves first; overlays resolve and paint in declared layer and
stack order. Overlay children inherit the window clip and explicit overlay
clips, not incidental clips at the portal's declaration site. Hit testing uses
the reverse of final paint order. A blocking or modal overlay contributes a
blocking hit region, so uncovered content cannot be hit through it.

Keyboard focus, accessibility focus, hover, and pointer capture are separate
states. A modal overlay creates a focus scope, traps keyboard traversal, and
records a restoration target. Closing it restores focus to the invoker when
that ID is still live and focusable, otherwise to the scope's deterministic
fallback. Non-modal overlays do not trap focus unless their widget contract
explicitly says so.

Pointer capture belongs to `(window, pointer, widget ID)`. It survives pointer
movement outside a widget and ordinary reordering, but ends on pointer release,
explicit release, cancellation, window focus loss/suspension, or when the
owner becomes removed, hidden, disabled, or belongs to a closed overlay.

## Invalidation, damage, and animation

Invalidation is a scheduling and optimization signal, not a correctness
mechanism. Every scheduled generation builds a logically complete current
scene. Cache hits are permitted only when their keys include every current
input needed to prove equivalence.

- Layout invalidation includes viewport, scale-dependent logical constraints,
  style, hierarchy, intrinsic metrics, text/image content, and any animation
  value that affects measurement or layout.
- Paint invalidation includes visual style, resolved geometry, resources,
  hover/press/focus state, scrolling, and paint-affecting animation values.
- Semantic invalidation includes semantic properties, hierarchy, state,
  actions, focus, effective bounds, and clipping.

The resolved traversal computes new bounds. Damage is the union needed to
replace the prior committed pixels with the current display list, including
old bounds for removed/moved content and new bounds for inserted/moved content,
all clipped to the relevant surfaces. Reading prior committed bounds for this
comparison is allowed; copying them into a current output is not. Resize-wide
invalidation and full redraw remain safe fallbacks, never prerequisites for
correct geometry.

The frame clock is sampled once before declaration. Animation state lives
outside the ephemeral tree and produces values for that sample time. An active
animation requests the next frame or timer deadline until settled. Animation
can affect layout, paint, or semantics and follows the corresponding
invalidation rules. Disappearing widgets stop being rendered immediately;
exit animation requires an explicitly declared retained transition element,
not an old render node kept implicitly alive.

## Scene commit and ownership

A committed scene is an immutable, generation-numbered bundle containing at
least resolved logical nodes, display-list data, effective clips, a hit-test
index, a semantic snapshot, focus order/scopes, cursor result, and damage. All
of those products come from the same current-frame resolved geometry and
replace their predecessors atomically.

The platform runtime must route `WindowId` to a per-window context. That
context owns:

- viewport and scale factor;
- pending events and response ledger;
- persistent widget/scroll/animation state;
- keyboard and accessibility focus plus pointer captures;
- the ephemeral builder while a frame is active;
- the last committed scene generation; and
- surface/renderer submission state and the accessibility adapter.

Widget IDs are scoped by window. Windows may share immutable fonts, decoded
images, renderer devices, and other resource caches, but never committed
geometry, input queues, focus, capture, animation clocks, or mutable widget
state. Closing a window cancels its captures and work without affecting any
other window.

ADR-004 will refine public multi-window APIs and resource lifetime. It may not
weaken this ownership boundary.

## Headless rendering

The UI-core phases depend on a viewport, logical scale, clock, intrinsic
measurer, input queue, and scene consumer—not on Winit, WGPU, a surface, or a
glyph atlas. The Gate 1 harness will provide deterministic fixtures and a null
consumer that records the committed scene as structured data. The same frame
pipeline must accept synthetic input and construct two independent window
contexts in one process.

This is the minimum harness pulled forward from Phase 2. It does not require
the Phase 2 crate split, production renderer abstraction, font discovery, or
serialization design.

## Reversal of the frame-1 cursor fallback

This ADR explicitly reverses the fallback accepted by the 2026-03-26
tree-primary layout decision. The present behavior paints a cursor-based
approximation when no `prev_layout` entry exists, then uses solved geometry on
a later frame. Newly inserted nodes can take the same fallback path.

Gate 1 supersedes that compromise. There is no cursor fallback and no warm-up
frame in the Phase 1 contract. The first frame and the first frame containing
an inserted or changed node must be measured, laid out, painted, hit-tested,
clipped, damaged, and exposed semantically from that generation's Taffy result.
The reversal is required because the fallback violates one committed geometry
per frame, makes a visible approximation part of correctness, and was shown
unnecessary by the current-frame spike without replaying application code.

## Gate 1 contract-test plan

Phase 1 will add a narrow headless integration-test harness around the new
frame core. It should use a deterministic text/image measurer, synthetic clock
and input queue, null scene consumer, and snapshot-friendly logical records.
It must not require a production crate split or a window/GPU initialization.

The required tests are:

| Test | Contract |
| --- | --- |
| `unchanged_first_and_second_frames_match` | The first committed scene equals an unchanged second scene, apart from generation/diagnostic fields; the declaration counter increments once per frame. |
| `resize_is_correct_in_first_post_resize_frame` | Flex/grid/text geometry and every derived record use the new viewport immediately. |
| `structural_changes_have_no_stale_geometry` | Add, remove, hide, and reorder in one generation update paint, hit, clip, semantic, focus-order, and damage records without stale nodes. |
| `metric_changes_reflow_and_paint_together` | Text and theme metric changes produce constrained layout and matching paint in one generation. |
| `one_node_supplies_all_resolved_bounds` | Paint, hit, effective clip, accessibility, and current damage bounds trace to the same resolved node/generation. |
| `scroll_offset_is_current_frame_state` | A changed offset updates transforms, clips, hits, semantics, and damage without a stale-layout exception. |
| `blocking_overlay_owns_topmost_hit` | Overlay paint order, clipping, focus scope, semantics, and reverse-order hit testing agree; content is not hit through the blocker. |
| `input_targets_committed_generation` | Synthetic input routes against generation N; its response is observed once during generation N+1 declaration, while a newly inserted target cannot receive it early. |
| `removed_capture_is_cancelled` | Removal/hiding of a captured or focused widget releases capture, synthesizes cancellation, and applies deterministic focus restoration before commit. |
| `headless_pipeline_has_no_platform_or_gpu` | The representative scene commits through the null consumer without Winit/WGPU or atlas construction. |
| `two_window_contexts_are_isolated` | Two contexts can commit different viewport, focus, capture, state, and scene generations without cross-window IDs or geometry. |
| `application_closure_is_never_replayed` | Measurement, layout, resolve, and commit cannot increment the declaration counter or invoke application callbacks. |

Tests should compare unrounded logical geometry. Renderer-boundary snapping
tests belong with typed coordinate work, not this harness. Taffy's upstream
suite owns CSS conformance; Esox tests only its style adapter and framework
contracts, including current defaults, zero-based grid mapping, constrained
measurement, scrolling, and overlays.

## Consequences

### Benefits

- First-frame and post-change correctness no longer depend on a warm-up frame.
- Paint, input, accessibility, clipping, and damage have one geometric source.
- Immediate-mode application ergonomics and once-only declaration are
  preserved.
- The frame core has a direct path to deterministic headless tests and
  per-window ownership.
- Persistent state remains possible without committing to a reconciled render
  tree.

### Costs and risks

- Widgets that currently paint during declaration must become lightweight
  declarations with a later paint traversal.
- Existing `Response` internals must separate committed-scene dispatch from
  declaration-time observation.
- Overlay, focus, capture, semantic, and damage logic must move onto resolved
  node identity rather than ad hoc paint-time rectangles.
- A complete per-frame tree may cost more than retained incremental work. Phase
  1 accepts that cost for correctness; measured caches or reconciliation may be
  added later without changing the source-of-truth contract.
- Application mutations after a widget call cannot retroactively alter already
  declared elements. This is explicit and may require a follow-up frame.

## Rejected alternatives

- **Paint while declaring, consume `prev_layout`:** fails first-frame and
  structural-change correctness.
- **Replay application closures after layout:** duplicates side effects and
  makes application code obey an implicit purity contract.
- **Recorded widget/paint command stream:** reconstructs the hierarchy Taffy
  already requires and adds an unnecessary intermediate representation.
- **Persistent reconciled render tree as the foundation:** adds reuse,
  removal, and move semantics to the correctness path without a demonstrated
  correctness benefit.
- **Dispatch against the tree under construction:** has no complete geometry,
  makes targeting declaration-order dependent, and cannot represent what the
  user actually saw when the event occurred.
