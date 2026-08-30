# Terminal presentation

Ghostty owns parsed cursor state. This sidecar updates a `GhosttyRenderState` from the terminal and
reads `CURSOR_VISUAL_STYLE` and `CURSOR_BLINKING`. It does not parse CSI or OSC in an adapter.

`TerminalCursorStyle` carries block, underline, or bar plus the engine's blink state. DECTCEM
visibility remains in `TerminalModes.show_cursor`; hiding the cursor does not erase its selected
style. Ghostty's hollow block is a focus presentation and maps to the block terminal shape.

Blink scheduling is renderer policy. This provider declares the 600 ms interval used by Ghostty's
renderer. The common terminal Kit schedules frames only while the semantic cursor is visible and
blinking.

The pinned libghostty-vt C API exposes raw OSC foreground, background and cursor overrides plus a
256-entry palette override mask. The Sidecar maps those values directly to
`TerminalThemeOverrides`; it does not infer override presence by comparing effective and default
colors. OSC reset returns no value or clears a mask bit, so the common renderer reveals the current
host base theme and repaints before publishing theme state.

Pointer reporting also stays in Ghostty. The sidecar owns one `GhosttyMouseEncoder` and reusable
`GhosttyMouseEvent` per terminal engine, refreshes encoder mode and format directly from the live
`GhosttyTerminal`, and passes action, button, modifiers, and cell position through the C API. The
adapter does not reconstruct X10, UTF-8, SGR, or motion bytes. `tests/pointer_input.rs` pins SGR
press, held motion, release, and no-button any-motion results.

Tracking-mode state follows the engine's exposed facts. DEC 9 is a native Ghostty mode and maps to
`TerminalModes.mouse_x10`. Ghostty does not recognize DEC 1001, so `mouse_highlight` is explicitly
false; it is not inferred from DEC 1000, 1002, or 1003. The Kit's public `reports_pointer` helper
admits only phases supported by the current mode before the native encoder is called. In
particular, an X10 press is admitted while an X10 release is refused at this boundary.

Wheel mouse reports reuse that public event and encoder with Ghostty buttons four through seven.
`tests/wheel_input.rs` pins SGR and legacy/UTF-8 output, both axes, repeated steps, position and
modifiers. The public `mouse_reporting` helper admits every live tracking mode; Ghostty's X10
encoder then truthfully refuses wheel buttons because its X10 implementation reports only primary
button presses. No alternate encoder or protocol alias is introduced. Alternate-screen mode 1007
owns a separate application-cursor-key route. Both routes validate the live modes again at the
engine boundary, so a mode change between Kit routing and encoding is refused. Device-unit
normalization, fractional accumulation and ordinary scrollback remain in the common terminal Kit;
there is no provider fallback. Selection remains on Ghostty's native selection-gesture and
formatter APIs and is pinned separately by `tests/selection.rs`.

`tests/conformance.rs::cursor_style` runs the contract-owned DECSCUSR, DEC mode 12, DECTCEM, and
warm rehydrate cases. `make verify TARGET=aarch64-apple-darwin` verifies only this provider and its
declared Ghostty SDK artifact.
