# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-30

- Wheel mouse reports now use Ghostty's public encoder refreshed from live terminal modes.
- SGR, default legacy and UTF-8 legacy routes preserve both axes, coordinates, modifiers and steps.
- Alternate-screen mode 1007 emits application cursor keys on both axes.
- Stale routes are refused; device normalization and ordinary scrollback remain Kit-owned.

## 2026-08-28

- Cursor shape and blink state now come from Ghostty's render-state API.
- The renderer receives Ghostty's 600 ms cursor animation policy.
- Contract cursor acceptance and the arm64 owner gate passed.
