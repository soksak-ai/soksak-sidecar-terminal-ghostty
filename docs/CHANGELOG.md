# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-30

### 0.0.40

- Pinned the Ghostty SDK source revision that strips checkout-specific debug paths and
  canonicalizes Darwin static-archive metadata and member order.
- Darwin Rust links now pass `-Wl,-no_uuid`, removing ld64's nondeterministic `LC_UUID` instead of
  patching a completed binary.
- Added named cross-root release comparison and proved two clean checkout releases byte-identical.

### 0.0.39 (unpublished candidate)

- Pinned the common terminal Kit to its final `v0.0.34` release commit
  (`20fb2d73d13e5bcde592380d3052c5d2204a592f`).
- Exposed Ghostty's native DEC 9 state as `mouseX10`; DEC 1001 remains explicitly unsupported
  instead of being aliased to normal, button-event, or any-event tracking.
- Wheel and pointer admission now use the Kit's public `mouse_reporting` and `reports_pointer`
  helpers while Ghostty's public encoder remains the sole owner of emitted bytes and refusals.

### 0.0.38

- Wheel mouse reports now use Ghostty's public encoder refreshed from live terminal modes.
- SGR, default legacy and UTF-8 legacy routes preserve both axes, coordinates, modifiers and steps.
- Alternate-screen mode 1007 emits application cursor keys on both axes.
- Stale routes are refused; device normalization and ordinary scrollback remain Kit-owned.

## 2026-08-28

- Cursor shape and blink state now come from Ghostty's render-state API.
- The renderer receives Ghostty's 600 ms cursor animation policy.
- Contract cursor acceptance and the arm64 owner gate passed.
