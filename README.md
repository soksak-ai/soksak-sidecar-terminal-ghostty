# soksak-sidecar-terminal-ghostty

The terminal-domain restore sidecar built on the **ghostty** VT engine (`libghostty-vt`).
It is an engine unit implementing the contract `soksak-spec-sidecar-terminal` — the same
contract the other engine units implement on their own engines. One contract, many
engine units, one at a time behind a terminal plugin's manifest declaration (NAMING §8:
the unit name carries the engine, exactly as `soksak-sidecar-browser-chromium` carries
Chromium).

## The contract lives elsewhere — this repo does not copy it

The normative wire (server face, consumer/daemon peering, checkpoint policy, failure
semantics, acceptance) is owned by **one** repo, and it is not this one:
`soksak-contract-terminal` (`~/.soksak-dev/contracts/soksak-contract-terminal`). It owns
`SPEC.md`, the corpus, the declared goldens, and the assertions this unit is graded by.
This unit implements that contract; it does not restate it.

## Build requirements

The engine is a C library, not a crate: `build.rs` links the static archive
`libghostty-vt.a`, and the seat (`src/engine.rs`) calls its C ABI. The archive is not
vendored into this repo — it is built from the ghostty source:

| Requirement | Value |
| --- | --- |
| Zig | **0.15.2** (ghostty's `minimum_zig_version`) |
| ghostty source | pinned at commit **`a887df4`** |
| Build command | `zig build -Demit-lib-vt=true -Doptimize=ReleaseFast` |
| Archive | `<ghostty>/zig-out/lib/libghostty-vt.a` |

The commit is pinned on purpose. libghostty-vt declares that its *functionality* is
stable but its *API signatures* are still in flux and may break without warning; the pin
is what keeps that churn out of this unit until a deliberate bump.

`build.rs` resolves the archive by declaration first (`SOKSAK_GHOSTTY_VT_LIB`), then by
the vendor convention (`../../vendor/ghostty/zig-out/lib`). It fails loudly with the
build command when the archive is absent — it never links silently against something
else. The engine's `lib` directory ships a dylib next to the archive and the macOS linker
prefers the dylib, so `build.rs` stages the archive alone into `OUT_DIR` and links that:
the sidecar binary carries the engine rather than hunting for a shared library at runtime.

## Engine seat vs shared machinery

The restore domain is engine-agnostic: the tee consumer `daemon.rs`, the checkpoint
policy `checkpoint.rs`, the mirror + ANSI serializer `mirror.rs`, the daemon wire
`proto.rs`, and the service runtime `service.rs`/`main.rs` never name an engine. The
engine lives behind one face in `engine.rs`, implemented here on `libghostty-vt`, exposing
`feed`/`resize`/grid·mode·cursor reads. A different engine unit swaps that one file; the
restore domain logic stays put.

## Graded against a declared golden, not against another engine

The contract declares the screen each corpus stream must produce, and this unit is graded
against that declaration: its mirror's screen must equal the golden, and the screen its own
restore paint rebuilds must equal the same golden. Nothing renders the paint on this unit's
behalf, and no engine's behaviour defines correctness — the standard is external to every
implementation, this one included.

## Engine specifics

**Answerback.** ghostty can actually answer queries, unlike an engine that has no reply
path at all. Left alone it discards them silently, but then a swallowed query is
unobservable. So the seat installs **count-then-drop** callbacks on the reply paths:
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

**This unit passes when `scripts/gate.sh` passes, and by no other means.** One command, all of
it blocking: the seven fixtures against the contract's declared goldens, the unit tests, the
real-daemon integration, and the performance budgets (SPEC.md §14.2). The benchmark is ignored
in the ordinary test run — it would slow the development loop — so the gate is what makes the
budget binding rather than decorative. The contract repo's own `scripts/gate.sh` runs this one
alongside the other units and adds the guard that only shows when they stand side by side.

## Acceptance

The contract's acceptance suite belongs to the kit, not to this repo. The seven engine-neutral
restore fixtures live in `soksak-kit-terminal-conformance`, and this unit stands its mirror up
against them in one line (`tests/conformance.rs`). GREEN on that shared suite is the unit's
gate — and with no copy here, there is nothing to drift. Real-daemon integration
(`tests/ptyd_integration.rs`, driven by `scripts/e2e/ptyd-integration.sh`) exercises the
tee→mirror→checkpoint round trip against an isolated `soksak-ptyd` binary.

## Qualification verdict

Conformance result against `soksak-spec-sidecar-terminal`: **7 of 7 fixtures pass**.

The seven fixtures — scrollback across a mid-escape ring cut, CJK width across a mid-UTF-8
cut, alt-screen with frozen primary scrollback, private-mode rehydrate beyond the ring
window, the replay guard, cold paint of an alt-screen TUI, and DEC line-drawing round trip
— all pass against the declared goldens. The lib unit tests, `service_down`, and the real-ptyd integration are GREEN.

Fixture ④ was RED on the first run and is worth recording: the restored scrollback held
588 rows against the original's 1000. That was the byte-budget/page-pruning behavior
described above, in the seat's configuration of the engine — not a gap in the engine and
not a defect in the restore domain. Fixing the seat turned it GREEN with the fixture
unchanged. Neutering the seat (a no-op `feed`) turns all seven RED, which is the evidence
that the suite grades the engine rather than passing vacuously.

Unlike the vt100 unit, no engine capability was missing: DEC Special Graphics ships in the
engine, so fixture ⑦ was GREEN from the first run.

## Licensing is per-unit

This unit ships the ghostty engine (MIT) and carries its `LICENSE` +
`THIRD-PARTY-NOTICES`. No license crosses between units. The conformance judge is a dev-dependency and
ships nowhere, so its Apache-2.0 does not reach this unit either.
