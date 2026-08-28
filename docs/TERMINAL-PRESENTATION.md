# Terminal presentation

Ghostty owns parsed cursor state. This sidecar updates a `GhosttyRenderState` from the terminal and
reads `CURSOR_VISUAL_STYLE` and `CURSOR_BLINKING`. It does not parse CSI or OSC in an adapter.

`TerminalCursorStyle` carries block, underline, or bar plus the engine's blink state. DECTCEM
visibility remains in `TerminalModes.show_cursor`; hiding the cursor does not erase its selected
style. Ghostty's hollow block is a focus presentation and maps to the block terminal shape.

Blink scheduling is renderer policy. This provider declares the 600 ms interval used by Ghostty's
renderer. The common terminal Kit schedules frames only while the semantic cursor is visible and
blinking.

`tests/conformance.rs::cursor_style` runs the contract-owned DECSCUSR, DEC mode 12, DECTCEM, and
warm rehydrate cases. `make verify TARGET=aarch64-apple-darwin` verifies only this provider and its
declared Ghostty SDK artifact.
