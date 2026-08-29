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
press, held motion, release, and no-button any-motion results. Selection and wheel support remain
separate open rows; an explicit refusal is not evidence for either.

`tests/conformance.rs::cursor_style` runs the contract-owned DECSCUSR, DEC mode 12, DECTCEM, and
warm rehydrate cases. `make verify TARGET=aarch64-apple-darwin` verifies only this provider and its
declared Ghostty SDK artifact.
