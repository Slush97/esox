# Gate 0.5 Status and Evidence

**Status:** Complete as a decision gate on 2026-07-11.

| Required decision | Outcome | Evidence |
| --- | --- | --- |
| Layout solver | Adopt Taffy 0.12.1 | `GATE_0_5_TAFFY_SPIKE.md` |
| Current-frame representation | Adopt a lightweight owned element tree | `GATE_0_5_CURRENT_FRAME_SPIKE.md` |
| Accessibility boundary | Esox-owned semantic tree adapted to AccessKit | `GATE_0_5_ACCESSKIT_SPIKE.md` |
| Unicode text boundary | Esox-owned interface backed by cosmic-text 0.19.0 | `GATE_0_5_TEXT_STACK_SPIKE.md` |

The two blocking review questions are answered:

1. Immediate-mode syntax is the application-facing API, not a requirement on
   the retained internal representation. The application declaration runs
   once into a current-frame owned tree.
2. AccessKit is an adapter over the Esox-owned serializable semantic model.

No Gate 0.5 blocking review question remains unanswered. Review question 8,
the exact Unicode and locale support promised for the first public release,
is still open and must be settled before that release contract. It does not
change the selected Phase 2 boundary, which is required to carry locale and
direction hints and use Unicode-conformant algorithms.

Gate 0.5 completion does not mark Gate 1 complete. The headless FrameCore
contract evidence exists, but production `Ui` still consumes `prev_layout`.
Production integration must preserve the decisions above: no previous-frame
geometry in FrameCore, no replay of application closures, and no broad `Ui`
restructure hidden inside the spikes. The staged production cutover is defined
in [the Gate 1 production integration plan](GATE_1_PRODUCTION_INTEGRATION_PLAN.md).
