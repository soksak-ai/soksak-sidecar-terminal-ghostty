use soksak_kit_sidecar_terminal::mirror::{
    CellSide, EngineSelectionPoint, SelectionKind, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_ghostty::engine::Engine;

#[test]
fn simple_drag_uses_ghostty_selection_gesture_and_formatter() {
    let marker = "SELECT_GHOSTTY_1234567890";
    let mut engine = Engine::new(40, 3);
    engine.feed(marker.as_bytes());

    TerminalEngine::selection_begin(
        &mut engine,
        SelectionKind::Simple,
        EngineSelectionPoint {
            line: 0,
            col: 0,
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .expect("begin Ghostty selection");
    TerminalEngine::selection_update(
        &mut engine,
        EngineSelectionPoint {
            line: 0,
            col: u16::try_from(marker.len()).unwrap(),
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .expect("update Ghostty selection");

    assert_eq!(
        TerminalEngine::selection_text(&engine).as_deref(),
        Some(marker),
    );
    assert_eq!(
        TerminalEngine::selection_range(&engine, 0),
        Some((0, u16::try_from(marker.len() - 1).unwrap())),
    );
}

#[test]
fn semantic_and_line_kinds_use_ghostty_selection_behaviors() {
    let mut semantic = Engine::new(40, 3);
    semantic.feed(b"alpha beta");
    TerminalEngine::selection_begin(
        &mut semantic,
        SelectionKind::Semantic,
        EngineSelectionPoint {
            line: 0,
            col: 7,
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .expect("begin semantic selection");
    assert_eq!(
        TerminalEngine::selection_text(&semantic).as_deref(),
        Some("beta"),
    );

    let mut line = Engine::new(40, 3);
    line.feed(b"  hello line  ");
    TerminalEngine::selection_begin(
        &mut line,
        SelectionKind::Line,
        EngineSelectionPoint {
            line: 0,
            col: 5,
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .expect("begin line selection");
    assert_eq!(
        TerminalEngine::selection_text(&line).as_deref(),
        Some("hello line"),
    );
}

#[test]
fn terminal_owned_selection_survives_output_and_extends_from_its_anchor() {
    let mut engine = Engine::new(40, 4);
    engine.feed(b"alpha beta");
    TerminalEngine::selection_begin(
        &mut engine,
        SelectionKind::Simple,
        EngineSelectionPoint {
            line: 0,
            col: 0,
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .unwrap();
    TerminalEngine::selection_update(
        &mut engine,
        EngineSelectionPoint {
            line: 0,
            col: 5,
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .unwrap();
    engine.feed(b"\r\nnext");
    assert_eq!(
        TerminalEngine::selection_text(&engine).as_deref(),
        Some("alpha")
    );

    TerminalEngine::selection_begin(
        &mut engine,
        SelectionKind::Extend,
        EngineSelectionPoint {
            line: 0,
            col: 10,
            side: CellSide::Right,
        },
        SelectionModifiers::default(),
    )
    .expect("extend native selection");
    assert_eq!(
        TerminalEngine::selection_text(&engine).as_deref(),
        Some("alpha beta"),
    );
}
