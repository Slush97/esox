# Gate 1 declaration coverage and legacy disposition

**Status:** C1/LC1 pre-cutover decision record, audited 2026-07-12 against
`feature/frame-core-production`.

This inventory is the input to the atomic Gate 1 cutover. `Migrate` means a
FrameCore-backed declaration must exist before the legacy method is removed.
`Delete` means remove the legacy implementation/export at cutover. `Defer` means
the capability may be absent from the public runtime after cutover and can be
reintroduced only on FrameCore; it does not authorize a legacy fallback.

## Decision summary

1. **Defer the whole `esox_ui::interpret` runtime, not the markup language.**
   Remove the optional `esox_ui::interpret` module and its `markup` feature at
   the cutover, while retaining the independent `esox_markup` AST/parser crate.
   Reintroduce an interpreter only after it targets `DeclarationUi`, returns
   explicit unsupported/declaration errors, and applies state changes through a
   retry-safe candidate store or action ledger. This is the smallest cutover-safe
   choice and preserves the strategic declarative language without preserving a
   second legacy renderer.
2. **Do not migrate the legacy table or virtual-scroll wrappers.** Their only
   non-archived production callers are the interpreter, and both callbacks emit
   empty labels. Removing the interpreter removes those callers; delete the
   wrappers, state, and exports after the existing production declarations cover
   real application callers.
3. **Migrate the useful leaf/container surface; delete and defer low-value
   parity.** Rating, drop-zone, spinner/skeleton parity, rich-text rendering, and
   the full animation API are absent at cutover. The demo is trimmed for spinner
   and moved to the new toast declaration; no legacy method remains just to keep
   a showcase compiling.
4. **Treat rich text and animation as explicit APIs, not incidental widget
   details.** Rich text needs an owned multi-span paint primitive plus post-Taffy
   wrap. Gate 1 only needs the minimal frame clock/repaint request used by cursor
   blink, tooltip delay, toast expiry, and any retained spinner; the public
   `animate*`, spring, keyframe, and `hover_t` APIs are deferred.

## Evidence and current coverage

- The production façade exposes containers (`column`, `row`, `grid`), uniform
  virtualization, table, text, image, solid/border paint, separator, progress,
  split panes, button, and checkbox in
  [`declaration.rs`](../crates/esox_ui/src/declaration.rs). Its module contract
  says declarations build an owned tree and resolve measurement/layout/paint/hit/
  semantics/damage after the application closure returns.
- Focused production regressions exist for declarations, checkbox, grid, image,
  overlay positioning, progress, scroll, separator, split pane, table, transforms,
  virtual scroll, and visibility under `crates/esox_ui/tests/production_*`.
- The legacy surface is 49 widget modules under `src/widgets/`, plus cursor
  layout, animation, toast, rich-text, focus, drag/drop, style, and accessibility
  helpers on `Ui`. Therefore existing production primitives are coverage
  building blocks, not public widget parity.
- `interpret` is optional but public under the `markup` feature
  ([`lib.rs`](../crates/esox_ui/src/lib.rs),
  [`Cargo.toml`](../crates/esox_ui/Cargo.toml)). Its five source files total 2,906
  lines. Its public `render` still accepts legacy `Ui`, and its dispatch matches
  all 74 current `WidgetKind` variants from `esox_markup`, primarily through
  legacy widget calls.
- Repository-wide non-archive search finds no interpreter consumer outside the
  module's own documentation/tests. `esox_markup` remains a workspace crate and
  has parser coverage independent of the renderer.
- `render_table` and `render_virtual_scroll` are the only non-archive callers of
  legacy `Ui::table`/`Ui::virtual_scroll` (apart from the table wrapper calling
  virtual scroll). Their callbacks deliberately render `ui.label("")` because
  static markup cannot provide row/cell content
  ([`render.rs`](../crates/esox_ui/src/interpret/render.rs)). Migrating those
  adapters would preserve no useful content.
- The interpreter already silently ignores `Popover`, `Tree`, `DropZone`,
  `Image`, and `Custom`, while rich text and style transitions call legacy rich
  text and `animate*` APIs. Keeping it during cutover would either retain legacy
  execution or silently misrepresent supported markup.
- The live demo calls toast APIs at seven sites and spinner once; it has no live
  rich-label, rating, drop-zone, spoiler, stepper, skeleton, table, or virtual
  scroll use. Archived showcase callers are not cutover blockers.

## Per-widget disposition

The production basis column names the existing declaration primitive or the
scheduled owner in `GATE_1_EXECUTION_PLAN.md`. “Delete legacy” is always part of
the atomic cutover after the migration condition is satisfied.

| Legacy widget module | Disposition | Production basis / cutover condition |
| --- | --- | --- |
| `accordion` | Migrate | LS7 disclosure group; then delete legacy module. |
| `alert` | Migrate | LS10 composition after LS1 paint vocabulary. |
| `avatar` | Migrate | LS10 composition; no retained legacy paint. |
| `badge` | Migrate | LS10 composition. |
| `blockquote` | Migrate | LS10 container/border composition. |
| `breadcrumb` | Migrate | LS10/O3 semantic navigation composition. |
| `button` | Migrate (covered base) | Existing `DeclarationUi::button`; variants become styles/composition. |
| `checkbox` | Migrate (covered) | Existing `DeclarationUi::checkbox`. |
| `chip` | Migrate | LS10 composition on button/text. |
| `code_block` | Migrate | LS10 plus LS11 wrapping/truncation; clipboard action must be transactional. |
| `collapsing` | Migrate | LS7 disclosure group. |
| `combobox` | Migrate | O10, after O5 anchored list and T5 text input. |
| `container` | Migrate | `DeclarationStyle`, row/column/grid, background/border; LS1 for remaining paint. |
| `drawer` | Migrate | O4 blocking-overlay declaration. |
| `drop_zone` | Delete + Defer | No live caller; reintroduce after pointer/file-drop ledger design. |
| `empty_state` | Migrate | LS10 composition. |
| `form` | Migrate | Composition after T5/T6 inputs and semantic-role merge O3. |
| `hyperlink` | Migrate | LS4 interactive leaf plus link semantics. |
| `icon` | Migrate | LS10 text/icon primitive composition. |
| `image` | Migrate (primitive covered) | Existing `DeclarationUi::image`; move cache/resource binding outside declaration. |
| `label` | Split | Plain variants use `text`; wrapped/truncated wait for LS11; `rich_label*` is Delete + Defer. |
| `menu_bar` | Migrate | O6 on O5 anchored-menu declaration. |
| `modal` | Migrate | O4 blocking-overlay declaration. |
| `number_input` | Migrate | LS9 spin control; T8 supplies transactional inline edit. Drop wheel increment. |
| `pagination` | Migrate | LS10 composition plus O3 roles/focus. |
| `paragraph` | Migrate | LS11 post-Taffy wrap resolution. |
| `popover` | Migrate | O5 on O1/O2/O3 anchored overlays. |
| `progress_bar` | Migrate (covered) | Existing `DeclarationUi::progress`; threshold variant becomes composition. |
| `radio` | Migrate | LS4 interactive leaf plus O3 radio semantics. |
| `rating` | Delete + Defer | No live caller; post-gate reintroduction on LS1 star primitive. |
| `scrollable` | Migrate | Current scroll offset/clip contract plus LC2 generic scrollbar declaration. |
| `select` | Migrate | O5 shared anchored-list declaration. |
| `separator` | Migrate (covered) | Existing `DeclarationUi::separator`. |
| `sidebar` | Migrate | O8 FrameCore container composition. |
| `skeleton` | Delete + Defer | No live caller; parity requires frame clock/continuous repaint. |
| `slider` | Migrate | LS5 transactional drag plus O3 slider semantics. |
| `spinner` | Delete + Defer | Trim the one live demo call; optional reintroduction after minimal clock. |
| `split_pane` | Migrate (covered) | Existing `split_pane_h/v` transactional declaration. |
| `spoiler` | Migrate | LS7 disclosure group; the demo does not gate it. |
| `status_bar` | Migrate | LS10 row/text composition. |
| `stepper` | Migrate | LS10/O3 navigation composition; no live demo dependency. |
| `table` | Migrate callers + Delete wrapper | Existing production table declaration; remove useless interpret caller, then legacy module/state/export (LC4/LC5). |
| `tabs` | Migrate | LS6 with keyboard navigation and O3 roles. |
| `text_area` | Migrate | T6 after retry-safe text ownership, editor paint, measurement, and IME ledger. |
| `text_input` | Migrate | T5 after T1/T3/T4/C2. |
| `toast` | Migrate | LS8 overlay; click dismiss first, expiry on LS2 minimal clock. Update the live demo. |
| `toggle` | Migrate | LS4 interactive leaf; visual transition may use minimal clock, not public `animate*`. |
| `tree` | Migrate | O9 after O3 roles/focus and LS3 pointer modifiers. |
| `virtual_scroll` | Migrate callers + Delete wrapper | Existing uniform `virtual_column`; remove useless interpret caller, add LC2 scrollbar, then delete legacy module/state/export. |

## Cross-cutting legacy API disposition

| Surface | Disposition | Condition |
| --- | --- | --- |
| `RichText`, `Span`, `FontWeight`, `Ui::rich_label*` | Delete runtime + Defer | Preserve/rework value vocabulary only with an owned multi-span paint primitive and wrap contract; do not flatten spans to plain text. |
| `Ui::animate*`, spring/keyframe APIs, `Ui::hover_t` | Delete + Defer | LS2 exposes only a frame clock and continuous-repaint request. Reintroduce general animation as candidate state, never `Instant` mutation during declaration. |
| Toast enqueue/render APIs and `ToastQueue` | Migrate | Enqueue becomes retry-safe intent/state; overlay is LS8 and expiry is LS2. |
| Cursor/measure/sub-region/layout helpers on `Ui` | Delete | Declaration styles and FrameCore resolution replace them; C9 source check forbids cursor/closure-measurement fallback. |
| Legacy focus, hit, a11y, disabled, clip, transform helpers | Delete | Use committed scene products and declaration/FrameCore ledgers. |
| Drag/drop helpers | Delete + Defer where unowned | Slider/split/tree use transactional pointer capture; generic payload/drop-zone returns post-gate. |

## Whole-module interpreter disposition

### Recommendation: preserve the language, retire the renderer at cutover

Deleting only the table/virtual arms is insufficient: every remaining render arm
still takes `&mut Ui`, style transitions mutate time-driven legacy state during
declaration, and `MarkupState` is mutated before a generation is known to commit.
Conversely, migrating 74 variants would put all deferred widgets back on the
critical path and would encourage silent fidelity loss.

At C4:

1. remove `pub mod interpret`, the `markup` feature, and the optional
   `esox_markup` dependency from `esox_ui`;
2. retain `crates/esox_markup` and its parser/AST tests unchanged;
3. remove legacy interpreter-only table/virtual state and callers before deleting
   the legacy widget modules;
4. do not move `interpret` into `archive/` as compilable fallback.

Post-gate, create a FrameCore-native interpreter in dependency batches rather
than translating the old renderer mechanically. Its public render operation
should declare into `DeclarationUi`, return `Result<Vec<Action>,
InterpretError>`, reject unsupported `WidgetKind` explicitly, and make state/
actions retry-safe. Suggested batches are: structure + plain text; simple leaves;
text inputs; overlays/navigation; rich text; virtual collections. Table and
virtual collection support should be redesigned around data/item providers, not
the current empty-label callbacks.

## Enforcement checks

C1 is complete when this inventory is accepted. C9 should add a checked source
gate equivalent to the following (exact paths may be refined during C4):

```sh
test ! -e crates/esox_ui/src/interpret
! rg -n 'pub mod interpret|feature = "markup"|Ui::begin|Ui::finish|prev_layout|layout_cache' \
  crates/esox_ui/src examples/demo/src
! rg -n 'pub fn (animate|animate_bool|animate_spring|animate_bool_spring|animate_keyframes|hover_t|virtual_scroll|table)\\b' \
  crates/esox_ui/src
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Before C4 begins, each `Migrate` row must either point to a passing production
contract test or be changed here to `Delete + Defer` with its live caller removed
in the same cutover window. No row may remain implicitly covered by legacy `Ui`.
