# Gate 1 execution plan and post-gate scope

**Status:** Scoped 2026-07-12 against the code on `feature/frame-core-production`.
**Sources:** [`GATE_1_PRODUCTION_INTEGRATION_PLAN.md`](GATE_1_PRODUCTION_INTEGRATION_PLAN.md) (progress record of same date), [`OXICORD_FRONTEND_PLAN.md`](OXICORD_FRONTEND_PLAN.md), six per-workstream code audits plus a cross-check pass.

Sizes: S = <1 day, M = 1–3 days, L = ~1 week, for one experienced solo Rust dev.

## Key findings

1. **Text-edit buffer ownership is the load-bearing decision.** FrameCore re-delivers
   keyboard responses after a rejected generation (`tests/keyboard_input_contract.rs:71-97`),
   but legacy text widgets apply edits to app-owned `InputState` during declaration —
   a retried generation double-inserts characters. Ownership (candidate-store text vs.
   edit intents keyed by `(committed_generation, dispatch_ordinal)`) must be decided and
   contract-tested before any text widget migrates. The choice propagates to
   number_input, combobox, and future table inline edit.
2. **No production `IntrinsicMeasurer` exists** — only test fixtures.
   `TextRenderer::new` requires a GPU context (`text.rs:181`) even though measurement is
   CPU-side. This is the earliest shared blocker (cutover gate criterion, wrapped labels,
   measured list). Single owner, staged: base GPU-free measurer first, then editor
   geometry queries (caret-x, x-to-byte, wrap lines) as a text-editing follow-on.
3. **FrameCore has zero IME support** (no queue, no preedit state, no composition
   presentation), and the platform never calls `set_ime_cursor_area`, so the candidate
   window is never positioned. IME commit text is an edit event and needs the same
   retry/clear-on-commit ledger semantics as keyboard input.
4. **Unowned work discovered:**
   - `interpret/` (~2,900 lines, 77 `WidgetKind` render arms, publicly exported) runs
     entirely on legacy `Ui` and breaks at cutover. Disposition is strategic — the
     AI-declarative layer is the product differentiator. Decision required pre-cutover.
   - Rich text (`rich_text.rs`, `Ui::rich_label*`) — claimed by no workstream, hard
     blocker for the Oxicord read-only shell. Needs its own post-gate workstream (L).
   - Post-Taffy wrap/truncation resolution for `PaintPrimitive::Text` — without it,
     `label_wrapped`/`paragraph`/`code_block` cannot migrate faithfully (M).
   - Generic scroll-container scrollbars (FrameCore has none; generalize the
     virtual/table scrollbar task to any scroll viewport).
   - Disposition of `Ui::animate*`/`hover_t` (Instant-driven, mutate state during
     declaration) — S decision required before the cutover commit can be written.
5. **Dependency cycle broken by deletion.** Cutover waited on legacy
   `virtual_scroll`/table caller migration; the wrapper task waited on cutover. The only
   live callers are two `interpret` sites with empty-label closures — skip the wrappers,
   delete the legacy implementations at the cutover commit.
6. **The cutover needs less than assumed.** Unmigrated widgets may be deleted from the
   public surface at cutover (the plan forbids fallback, not deletion). The demo does
   not use `rich_label`, `rating`, `drop_zone`, `spoiler`, `stepper`, or `skeleton` —
   all deferrable. Demo uses toast and spinner: either migrate on the minimal frame
   clock or trim the demo.
7. **Extracted shared engine tasks** so nothing large blocks unrelated work:
   - minimal frame clock + continuous-repaint request through the redraw-serial
     boundary (cursor blink, spinner, tooltip delay, toast auto-dismiss) — the full
     animation system stays post-gate;
   - pointer modifiers on `InputResponse` (`frame_core.rs:736-751` has none; tree and
     table multi-select need ctrl/shift);
   - one merged `SemanticRole`/`SemanticProperties` extension commit (overlays-nav and
     leaf-sweep both extend the same snapshot-tested enum).
8. **Contract decisions to record rather than build:** drop number_input
   wheel-increment (contradicts committed wheel routing, plan L137-152); record the
   virtual-collection/table accessibility deferral in the Gate 1 closure notes; the
   measured list should carry collection-size semantics from day one.

## Workstream task breakdown

### text-editing

| # | Task | Size | Depends on |
| --- | --- | --- | --- |
| T1 | Decide + contract-test transactional ownership of the editing buffer (forced-retry test, no double-apply) | M | — |
| T2 | Editor geometry queries on the renderer-neutral text boundary (caret-x, x-to-byte, wrap lines, line height) | M | C2 |
| T3 | Add IME events to the FrameCore ledger + per-window composition state | M | T1 |
| T4 | `EditorText` paint primitive with post-Taffy caret/selection/scroll resolution | L | T1, T2 |
| T5 | Single-line `text_input` declaration (checkbox/table pattern; Tab-consumption rule shared with O3) | L | T3, T4, C2 |
| T6 | Multiline `text_area` declaration (wrap layout, retained scroll, wheel routing) | L | T5 |
| T7 | Platform IME plumbing: `set_ime_cursor_area` from committed caret, per-focus enablement, unified clipboard path | M | T5 |
| T8 | number_input inline-edit integration (combobox reassigned to overlays-nav) | M | T5 |

### overlays-nav

| # | Task | Size | Depends on |
| --- | --- | --- | --- |
| O1 | Anchored overlay positioning (`anchored_to(anchor, placement, gap)`, resolve-time, flip/shift) | L | — |
| O2 | Overlay layer hoisting (portal ownership) in resolve traversal | M | — |
| O3 | Merged `SemanticRole` extension + focus-scope Tab/Shift-Tab traversal (owns the leaf-sweep role additions too) | M | — |
| O4 | Drawer + modal on blocking-overlay declarations (backdrop dismiss, Escape via ledger) | M | O2, O3 |
| O5 | Shared anchored-list declaration; migrate select, popover, context menu | L | O1, O2, O3 |
| O6 | menu_bar on the anchored menu declaration | M | O5 |
| O7 | Tooltip as non-blocking anchored overlay (delay timer on minimal frame clock) | M | O1, O2, clock |
| O8 | Sidebar composition onto FrameCore containers | M | — |
| O9 | Tree nodes with inter-node keyboard navigation (single owner; needs pointer modifiers) | L | O3 |
| O10 | Combobox = production text input + anchored list | L | O5, T5 |
| O11 | Command palette / quick-switcher primitive | M | O4, O10 |

### legacy-callers

| # | Task | Size | Depends on |
| --- | --- | --- | --- |
| LC1 | Decide interpret fate for Table/VirtualScroll (extend to whole-module interpret disposition) | S | — |
| LC2 | Scrollbar declaration parts — generalized to any scroll viewport (scrollable, virtual_column, table) | M | — |
| LC3 | Caller-parity gaps: weighted/auto columns, multi-select, sort-cycle helper (shrinks under deletion path) | M | LC1 |
| LC4 | Migrate or remove interpret `render_table`/`render_virtual_scroll` | S | LC1 |
| LC5 | Delete legacy virtual_scroll/table implementations, state, exports (wrapper task skipped — cycle break) | S | LC4 |

### cutover

| # | Task | Size | Depends on |
| --- | --- | --- | --- |
| C1 | Declaration coverage inventory + enforcement check; per-widget disposition (migrate/delete); records interpret, rich-text, animation-API, toast dispositions | S | — |
| C2 | GPU-independent production text/image measurer (single owner of the merged measurer task) | M | — |
| C3 | Embed FrameCore in `UiState`, reroute key/mouse/wheel/IME entry points | M | C2 |
| C4 | Atomic cutover: `Ui::begin/finish` → `begin_generation`/`finish_generation`; delete `prev_layout`/`layout_cache`/cursor fallback; demo trimming for deferred widgets in the same commit | L | C1, C3, T5–T6, O4–O6, LC5, leaf batches |
| C5 | Damage/redraw integration at the renderer boundary (consume `CommittedScene.damage`) | M | C4 |
| C6 | Update examples/demo to the cutover pipeline | M | C4 |
| C7 | Retire `layout_tree.rs` (1,624 lines) + cursor layout machinery | S | C4 |
| C8 | A11y output switch to committed semantic snapshot; adapt/feature-gate atspi | M | C4 |
| C9 | Run validation matrix, add source check (no prev_layout/cursor fallback/closure measurement), record Gate 1 closed | S | C4–C8 |

### leaf-sweep

| # | Task | Size | Depends on |
| --- | --- | --- | --- |
| LS1 | Extend `PaintPrimitive` vocabulary (rounded border, dashed border, side stripe, star, drop shadow) | M | — |
| LS2 | Minimal frame clock + continuous-repaint request (split out of the animation system) | S/M | — |
| LS3 | Pointer modifiers on `InputResponse` (split out of tree task) | S | — |
| LS4 | Simple interactive batch: toggle, radio, hyperlink | M | O3 |
| LS5 | Slider with transactional drag (split_pane pattern) | M | O3 |
| LS6 | Tabs with keyboard navigation | M | O3 |
| LS7 | Disclosure group: collapsing header, accordion, spoiler | M | O3 |
| LS8 | Toast overlay (click-dismiss now; auto-dismiss on LS2) | M | LS1, LS2 |
| LS9 | Number input spin control (inline edit stays stubbed until T8) | M | O3 |
| LS10 | Pure-composition sweep (~15 widgets; avatar/code_block/alert first per Oxicord) | L | LS1, LS4 |
| LS11 | Post-Taffy wrap/truncation for `PaintPrimitive::Text` (new, previously unowned) | M | C2 |
| LS12 | Deferrable past cutover via deletion: rating, drop_zone, spinner/skeleton parity, full animation/time source | — | post-gate |

### virt-list (post-gate track V — fully headless, runs in parallel throughout)

| # | Task | Size | Depends on |
| --- | --- | --- | --- |
| V1 | Keyed logical item identity (`virtual_item_key_id`; coordinate wrapper-ID rule with LC2/table) | S | — |
| V2 | Transactional measured-height cache with prefix sums (candidate-cloned `MeasuredListState`) | M | V1 |
| V3 | `plan_measured_list` + `measured_column` declaration (binary-search visible range, overscan, scroll-to-key) | L | V1, V2 |
| V4 | Commit-time measured-height write-back + convergence policy | M | V3 |
| V5 | Anchor model + prepend compensation (anchor key + intra-item offset) | L | V2, V3 |
| V6 | Bottom-follow mode with scroll-away disengage | M | V5 |
| V7 | Deterministic focus/input for keyed items entering/leaving range | S | V1, V3 |
| V8 | Headless contract suite: prepend, resize, reflow, deletion, large histories | M | V4–V7 |
| V9 | Reconcile uniform path; document table extension seam; collection-size semantics from day one | S | V3 |

## Execution order

- **Group A (start now, parallel):** T1, C1, C2, LC1, O3 (merged semantics), LS1, O2, O8. Track V starts (V1).
- **Group B:** LS2 (clock), LS3 (modifiers), LS11 (text wrap resolution), T2, O1, LS4–LS7, LC2. Track V: V2.
- **Group C:** T3, T4, O4, O5, O7, LS10, LC3, LC4. Track V: V3.
- **Group D:** T5, O6, O9, LS8. Track V: V4, V5.
- **Group E:** T6, T7, T8, LS9, O10, LC5. Track V: V6, V7.
- **Group F (serial cutover):** C3 → C4 (+C6 same commit window).
- **Group G (post-cutover, parallel):** C5, C8, C7 → C9 (Gate 1 closed). Track V: V8, V9.
- **Post-gate:** rich-text spans primitive + `rich_label` migration (Oxicord shell
  blocker), full animation/time source, spinner/skeleton parity, rating/drop_zone
  reintroduction, O11 command palette, mention-autocomplete (design the anchored-overlay
  API to accept caret-rect anchors now — S note).

## Sizing

~13 S / ~32 M / ~13 L across all workstreams before deferrals. The Gate-1-critical
subset (groups A–G minus track V and post-gate items) is roughly 2.5–4 solo months;
track V and much of groups B–C parallelize across agents.
