# soksak-sidecar-terminal-ghostty

The terminal-domain restore sidecar built on the **ghostty** VT engine (`libghostty-vt`).
It is an engine unit implementing the contract `soksak-spec-sidecar-terminal` — the same
contract the other engine units implement on their own engines. One contract, many
engine units, one at a time behind a terminal plugin's manifest declaration (NAMING §8:
the unit name carries the engine, exactly as `[redacted]` carries
Chromium).

## The contract lives elsewhere — this repo does not copy it

The normative wire (server face, consumer/daemon peering, checkpoint policy, failure
semantics, acceptance) is owned by **one** repo, and it is not this one:
`soksak-contract-terminal`. It owns
`SPEC.md`, the corpus, the declared reference states, and the assertions this unit is graded by.
This unit implements that contract; it does not restate it.

## Build requirements

The engine is a C library, not a crate: `build.rs` links the static archive
`libghostty-vt.a`, and the seat (`src/engine.rs`) calls its C ABI. The archive is not
vendored into this repo. `build-dependencies.json` is the single owner of the source repository,
exact commit, Zig version, supported target triples, and target-namespaced outputs. Make consumes
that declaration; neither the workflow nor `build.rs` repeats its values.

```sh
make prepare TARGET=aarch64-apple-darwin
make build TARGET=aarch64-apple-darwin
make verify TARGET=aarch64-apple-darwin
make stage TARGET=aarch64-apple-darwin OUT=dist
```

`prepare` materializes an exact clean source checkout, builds ReleaseFast with a portable CPU
baseline, writes the target archive and provenance files, and verifies a byte receipt. Repeating
the command revalidates and reuses the same output. A Linux archive built without ReleaseFast measured
0.482 MB/s against 79.218 MB/s daemon demand and lost 65,615,783 bytes; the pinned ReleaseFast
archive measured 125.572 MB/s, zero gap bytes and a visible tail marker on 2026-08-22.

`build.rs` accepts only the Make-owned build-dependency root, selects the receipt matching Cargo's
exact target, and rejects a missing or symbolic archive. It never accepts a raw library path,
guesses a checkout, or links silently against another installation. The engine's `lib` directory ships a dylib next to the archive and the macOS linker
prefers the dylib, so `build.rs` stages the archive alone into `OUT_DIR` and links that:
the sidecar binary carries the engine rather than hunting for a shared library at runtime.

## Engine seat vs shared machinery

The restore domain is engine-agnostic: the tee consumer `daemon.rs`, the checkpoint
policy `checkpoint.rs`, the mirror + ANSI serializer `mirror.rs`, the daemon wire
`proto.rs`, and the service runtime `service.rs`/`main.rs` never name an engine. The
engine lives behind one face in `engine.rs`, implemented here on `libghostty-vt`, exposing
`feed`/`resize`/grid·mode·cursor reads. A different engine unit swaps that one file; the
restore domain logic stays put.

## Graded against the declared reference state

The contract declares the screen each corpus stream must produce, and this unit is graded
against that declaration: its mirror's screen must equal the reference state, and the screen its own
restore paint rebuilds must equal the same reference state. Nothing renders the paint on this unit's
behalf. The declared reference state is the sole correctness criterion for this implementation.

## Engine specifics

**Answerback.** Ghostty exposes reply callbacks for terminal queries. The seat installs
**count-then-drop** callbacks so swallowed queries remain observable while no reply byte leaves
the mirror:
`write_pty` receives DSR, DECRQM, ENQ and OSC color-query responses; `device_attributes`
receives DA1/DA2/DA3 and returns false (answer nothing). Both count and discard the bytes,
which is the observability the contract asks for (`suppressedReplies`) with the invariant
intact: **no byte leaves the mirror**. One nuance is honest to record — an OSC color query
against this mirror produces no reply *to* suppress, because the mirror is not a display
and has no theme configured, so the engine has nothing to report. The live session's single
responder is the front terminal, which does have colors.

**Private modes.** Every mode the contract restores has a native getter (`mode_get`), so
nothing is reconstructed by observing unhandled sequences. The engine's fresh-terminal
defaults for the on-by-default modes (wraparound 7, cursor-visible 25, alternate-scroll
1007) match the judge's, which is what makes the serializer's "emit only what differs from
a fresh terminal" rule mean the same thing on both sides.

**Scrollback.** History is random-access through the `history` coordinate space, so grid
reads are pure — the seat never moves a read cursor and restores it, and its grid reads
take `&self`.

**Scrollback budget.** The engine's scrollback limit is a **byte** budget, not a line
count (the C header's "number of lines" wording notwithstanding), and pruning drops the
oldest **whole page**. A budget sized to the restore window therefore collapses below the
window the moment pruning fires: with heavy rows (wide characters, a distinct truecolor per
cell) the retained history fell from 1178 rows to 588 — one page gone — and the restored
screen came out shorter than the original. The seat gives the engine a byte budget that
covers the window with a page to spare, and reports the contract window (the newest
`MIRROR_SCROLLBACK_LINES` rows) from `history_size()` while indexing against the rows
actually retained. `engine_retains_the_whole_window_under_heavy_content` pins it.

**Character sets.** DEC Special Graphics translation happens at print time, so the cell
codepoint already holds the box glyph (`─│┌┐└┘`). The seat has no charset state to carry:
it reads the translated glyph straight out of the grid.

**Grid width.** A wide character is a body cell plus a spacer cell (tail/head), aligned
with the contract's canonical two-cell layout. A cell with a background but no text carries
its color in the cell content rather than in a style, so the seat reads both places —
reading only the style would lose the background of blank colored regions.

## The gate

**This unit passes when `make verify TARGET=<native-target>` passes, and by no other means.** One command, all of
it blocking: the seven fixtures against the contract's declared reference states, the unit tests, and
the performance budgets (SPEC.md §14.2). The benchmark is ignored
in the ordinary test run — it would slow the development loop — so the gate is what makes the
budget binding rather than decorative. The contract repo invokes this owner command
alongside the other units and adds the guard that only shows when they stand side by side.

## Acceptance

The contract's acceptance suite belongs to the kit, not to this repo. The seven engine-neutral
restore fixtures live in `soksak-kit-terminal-conformance`, and this unit stands its mirror up
against them in one line (`tests/conformance.rs`). GREEN on that shared suite is the unit's
gate — and with no copy here, there is nothing to drift. Installed PTY and recovery-sidecar
composition belongs to the terminal acceptance repository, which installs both products through
Core and verifies warm and archived restore across every terminal plugin.

## Qualification verdict

Conformance result against `soksak-spec-sidecar-terminal`: **7 of 7 fixtures pass**.

The seven fixtures — scrollback across a mid-escape ring cut, CJK width across a mid-UTF-8
cut, alt-screen with frozen primary scrollback, private-mode rehydrate beyond the ring
window, the replay guard, cold paint of an alt-screen TUI, and DEC line-drawing round trip
— all pass against the declared reference states. The lib unit tests, `service_down`, and the real-ptyd integration are GREEN.

Fixture ④ was RED on the first run and is worth recording: the restored scrollback held
588 rows against the original's 1000. That was the byte-budget/page-pruning behavior
described above, in the seat's configuration of the engine — not a gap in the engine and
not a defect in the restore domain. Fixing the seat turned it GREEN with the fixture
unchanged. Neutering the seat (a no-op `feed`) turns all seven RED, which is the evidence
that the suite grades the engine rather than passing vacuously.

DEC Special Graphics designation and invocation are available at this engine boundary, so
fixture ⑦ was GREEN on its first run.

## Licensing is per-unit

This unit ships the ghostty engine (MIT) and carries its `LICENSE` +
`THIRD-PARTY-NOTICES`. No license crosses between units. The conformance judge is a dev-dependency and
ships nowhere, so its Apache-2.0 does not reach this unit either.
