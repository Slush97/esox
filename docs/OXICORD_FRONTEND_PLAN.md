# Oxicord-powered Discord frontend plan

**Status:** Directional target; implementation has not started.

**Purpose:** Use a native Discord frontend as a forcing function for Esox's
production UI architecture without bypassing the gated stabilization plan or
coupling FrameCore to Discord-specific types.

The intended product is an Esox presentation layer over Oxicord's domain,
application, and infrastructure capabilities. Oxicord's Ratatui presentation
layer is not part of the target UI. Esox remains a general-purpose toolkit;
every prerequisite added to it must be justified as a reusable UI capability.

## Delivery horizons

These are planning ranges for one focused experienced developer, not schedule
commitments:

| Milestone | Expected range | Meaning |
| --- | ---: | --- |
| Disposable legacy-`Ui` mock-up | 1–3 weeks | Static or lightly interactive shell; not a production foundation |
| FrameCore vertical slice | 4–8 focused weeks after its prerequisites | Authenticate, select a channel, view messages, and send text |
| Usable alpha | 2–4 months | History, composition, images, notifications, and dependable navigation |
| Polished daily client | 6–12+ months | Accessibility, resilience, rich content, settings, and broad interaction coverage |

Do not build the production client on legacy previous-frame geometry merely to
reach the first horizon. A disposable visual exploration is allowed only when
clearly isolated from the product implementation.

## Product slices

### Slice A — Read-only Discord shell

- authenticate through the selected Oxicord boundary;
- display guilds, channels, and a selected channel's messages;
- process live message, presence, typing, and unread updates;
- preserve selection and scroll state across updates; and
- keep network work off the rendering thread.

### Slice B — Text participation

- send and edit messages;
- multiline composition, selection, clipboard, IME, and undo/redo;
- reply context, mentions, and autocomplete;
- focus-correct keyboard shortcuts; and
- optimistic send state with visible failure and retry.

### Slice C — Rich participation

- Markdown, links, code blocks, emoji, reactions, and spoilers;
- avatars, attachments, image previews, and external opening;
- history prepend without viewport jumps;
- desktop notifications and unread navigation; and
- file selection/upload with progress and cancellation.

### Slice D — Daily-driver quality

- reconnect, resume, rate-limit, offline, and renderer-loss behavior;
- settings, theming, keybindings, and account safety;
- screen-reader output, complete keyboard navigation, and focus indicators;
- performance budgets for large guilds, channels, and histories; and
- packaging, update, diagnostics, and support documentation.

Voice/video, screen sharing, activities, and plugin compatibility are outside
the initial product scope.

## Esox prerequisites

### P0 — Complete Gate 1

Finish the atomic production `Ui` cutover described in
[`GATE_1_PRODUCTION_INTEGRATION_PLAN.md`](GATE_1_PRODUCTION_INTEGRATION_PLAN.md):

- one FrameCore owner per production window;
- no previous-frame geometry as a correctness source;
- production keyboard delivery through the transactional ledger;
- production table and virtual-content caller migration; and
- the full cutover validation matrix.

The Discord frontend must not introduce a mixed legacy/FrameCore committed
scene.

### P1 — Variable-height anchored virtualization

Generalize uniform virtual content into a reusable measured-list primitive:

- stable logical item identity independent of visible slots;
- cached or estimated item heights corrected by current measurements;
- efficient prefix sums and visible-range lookup;
- scroll anchoring when older items are prepended or heights change;
- explicit bottom-follow mode that disengages when the user scrolls away;
- deterministic focus and input behavior for items entering/leaving the range;
- retry-safe candidate state under FrameCore transactions; and
- headless tests for prepend, resize, reflow, deletion, and large histories.

This is the highest-value missing primitive for a chat client and should remain
message-model agnostic.

### P2 — Production text and media

- migrate multiline text editing onto FrameCore keyboard/focus routing;
- preserve per-event text, repeat, modifier, selection, and IME semantics;
- add production rich-text spans, links, code blocks, wrapping, and selection;
- submit image primitives through the production renderer boundary;
- support decoded-image measurement, caching, loading, failure, and damage; and
- make resource completion request the correct window's next generation.

### P3 — Production navigation and overlays

- migrate tree/sidebar, menu, tooltip, popover, modal, and context-menu needs;
- provide focus scopes, focus-visible state, directional navigation, and
  keyboard activation consistently;
- add command/quick-switcher and mention-autocomplete primitives; and
- ensure overlays, virtual content, and keyboard focus compose atomically.

### P4 — Async application bridge

Define a reusable platform/application boundary rather than embedding Tokio in
widgets:

- background tasks publish owned application events;
- events are batched into one window-local model update;
- the live window incarnation is woken exactly once for pending work;
- cancellation and shutdown cannot target a destroyed window;
- secrets never enter UI snapshots, tracing, or diagnostic dumps; and
- deterministic tests use a fake event source and clock.

## Oxicord integration boundary

Before frontend implementation:

1. Pin a reviewed Oxicord revision.
2. Resolve the repository's license ambiguity: its GitHub repository advertises
   GPL-3.0 while its package manifest currently declares MIT.
3. Decide whether to depend on Oxicord as a library, maintain a narrow adapter,
   or extract upstream-neutral crates in collaboration with its maintainers.
4. Keep Oxicord domain/application/infrastructure types behind an adapter owned
   by the frontend application, not inside Esox crates.
5. Document the risks of unofficial Discord clients, user-token handling, and
   Discord terms before distributing builds.

The adapter should expose application concepts such as snapshots, commands,
and event streams. Esox should receive presentation models with stable IDs and
owned display data, never transport-layer gateway payloads.

## First vertical-slice architecture

```text
Oxicord gateway/REST/keyring
            |
            v
frontend adapter + application model
            |
       owned event batch
            |
            v
window-local Esox FrameCore owner
            |
            v
committed scene -> WGPU renderer
```

The first real frontend slice is complete when it can:

1. start without blocking the UI thread;
2. authenticate without logging or serializing the token;
3. show a guild/channel sidebar from live application state;
4. render a variable-height channel history;
5. retain a stable viewport while older history is prepended;
6. follow new messages only while bottom-follow mode is active;
7. compose and send a multiline text message;
8. route keyboard input through committed focus;
9. reconnect or display a recoverable failure state; and
10. pass headless lifecycle tests with a fake Oxicord adapter.

## Near-term steering rules

Until the vertical slice starts, prefer Gate 1 work that advances these generic
capabilities:

1. atomic production `Ui` ownership and cutover;
2. production keyboard navigation and text input;
3. variable-height anchored virtualization;
4. rich text and image submission;
5. async window-local application events; and
6. tree/sidebar and overlay migration.

Avoid Discord-specific widgets in `esox_ui`. A message row, guild rail, or
channel tree belongs in the frontend application and should be composed from
general primitives.

## Readiness checkpoints

### Ready to start the frontend repository or crate

- Gate 1 cutover is green;
- the Oxicord revision and licensing strategy are recorded;
- the adapter can replay a deterministic fixture without networking; and
- a window-local async event can safely request a FrameCore generation.

### Ready for the read-only shell

- measured anchored virtualization passes its contract suite;
- rich text and image resources render through production scene submission;
- sidebar/tree interactions use production focus and input; and
- live updates do not cause duplicate declaration or viewport jumps.

### Ready for alpha distribution

- text composition and upload flows are recoverable;
- secrets use a supported keyring path;
- reconnect, rate-limit, and shutdown behavior are tested;
- minimum keyboard and accessibility navigation is documented; and
- the unofficial-client risk is disclosed prominently.

## Explicitly deferred decisions

- final product name and repository layout;
- whether the frontend is distributed under GPL-compatible terms;
- voice/video architecture;
- multi-account and multi-window behavior;
- mobile, web, and non-Linux targets; and
- whether upstream Oxicord accepts presentation-independent library changes.
