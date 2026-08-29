use soksak_kit_sidecar_terminal::mirror::{
    EnginePointerInput, PointerButton, PointerPhase, SelectionModifiers,
};
use soksak_sidecar_terminal_ghostty::engine::Engine;

fn pointer(phase: PointerPhase, button: PointerButton) -> EnginePointerInput {
    EnginePointerInput {
        row: 2,
        col: 1,
        phase,
        button,
        click_count: if phase == PointerPhase::Move { 0 } else { 1 },
        modifiers: SelectionModifiers::default(),
    }
}

#[test]
fn ghostty_encoder_owns_sgr_press_drag_release_and_free_motion() {
    let mut engine = Engine::new(120, 40);
    engine.feed(b"\x1b[?1002h\x1b[?1006h");
    assert_eq!(
        engine.pointer_input(pointer(PointerPhase::Down, PointerButton::Left)).unwrap(),
        b"\x1b[<0;2;3M",
    );
    assert_eq!(
        engine.pointer_input(pointer(PointerPhase::Move, PointerButton::Left)).unwrap(),
        b"\x1b[<32;2;3M",
    );
    assert_eq!(
        engine.pointer_input(pointer(PointerPhase::Up, PointerButton::Left)).unwrap(),
        b"\x1b[<0;2;3m",
    );

    engine.feed(b"\x1b[?1002l\x1b[?1003h");
    assert_eq!(
        engine.pointer_input(pointer(PointerPhase::Move, PointerButton::None)).unwrap(),
        b"\x1b[<35;2;3M",
    );
}
