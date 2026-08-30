use soksak_kit_sidecar_terminal::mirror::{
    EngineWheelInput, EngineWheelRoute, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_ghostty::engine::Engine;

fn wheel(horizontal: i32, vertical: i32, route: EngineWheelRoute) -> EngineWheelInput {
    EngineWheelInput {
        row: 2,
        col: 1,
        horizontal,
        vertical,
        modifiers: SelectionModifiers::default(),
        route,
    }
}

#[test]
fn ghostty_native_encoder_owns_sgr_mouse_wheel_direction_position_and_repetition() {
    let mut engine = Engine::new(120, 40);
    engine.feed(b"\x1b[?1000h\x1b[?1006h");

    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(0, -2, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<64;2;3M\x1b[<64;2;3M",
    );
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(-1, 1, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<65;2;3M\x1b[<66;2;3M",
    );
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(1, 0, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<67;2;3M",
    );
}

#[test]
fn ghostty_native_encoder_owns_legacy_and_utf8_mouse_wheel_encodings() {
    let mut engine = Engine::new(240, 120);
    engine.feed(b"\x1b[?1000h");
    let mut legacy = wheel(0, -1, EngineWheelRoute::MouseReport);
    legacy.modifiers = SelectionModifiers {
        shift: true,
        alt: true,
        control: true,
        meta: false,
    };
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, legacy).unwrap(),
        [0x1b, b'[', b'M', 124, 34, 35],
    );
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(-2, 1, EngineWheelRoute::MouseReport),)
            .unwrap(),
        [
            0x1b, b'[', b'M', 97, 34, 35, 0x1b, b'[', b'M', 98, 34, 35, 0x1b, b'[', b'M', 98, 34,
            35,
        ],
    );

    engine.feed(b"\x1b[?1005h");
    let mut extended = wheel(0, -1, EngineWheelRoute::MouseReport);
    extended.col = 100;
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, extended).unwrap(),
        [0x1b, b'[', b'M', 96, 0xc2, 0x85, 35],
    );
}

#[test]
fn ghostty_owns_alternate_screen_alternate_scroll_on_both_axes() {
    let mut engine = Engine::new(80, 24);
    engine.feed(b"\x1b[?1049h\x1b[?1007h");

    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(1, -2, EngineWheelRoute::AlternateScroll),)
            .unwrap(),
        b"\x1bOA\x1bOA\x1bOC",
    );
}

#[test]
fn ghostty_rejects_wheel_routes_when_live_modes_no_longer_select_them() {
    let mut engine = Engine::new(80, 24);
    engine.feed(b"\x1b[?1000h\x1b[?1000l");
    let mouse_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::MouseReport))
            .unwrap_err();
    assert!(
        mouse_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{mouse_error}"
    );

    engine.feed(b"\x1b[?1049h\x1b[?1007h\x1b[?1007l");
    let alternate_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::AlternateScroll))
            .unwrap_err();
    assert!(
        alternate_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{alternate_error}"
    );

    engine.feed(b"\x1b[?1007h\x1b[?1000h");
    let precedence_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::AlternateScroll))
            .unwrap_err();
    assert!(
        precedence_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{precedence_error}"
    );
}

#[test]
fn ghostty_reports_x10_but_not_unsupported_highlight_as_native_engine_facts() {
    let mut engine = Engine::new(80, 24);
    engine.feed(b"\x1b[?9h");

    let x10 = TerminalEngine::modes(&engine);
    assert!(x10.mouse_x10);
    assert!(!x10.mouse_click && !x10.mouse_highlight && !x10.mouse_drag && !x10.mouse_motion);
    assert!(x10.mouse_reporting());

    let x10_wheel_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::MouseReport))
            .unwrap_err();
    assert!(
        x10_wheel_error.starts_with("WHEEL_ENCODER_REFUSED:"),
        "the public mode helper must admit the route before Ghostty's X10 encoder refuses wheel buttons: {x10_wheel_error}"
    );

    engine.feed(b"\x1b[?9l\x1b[?1001h");
    let unsupported_highlight = TerminalEngine::modes(&engine);
    assert!(!unsupported_highlight.mouse_x10);
    assert!(!unsupported_highlight.mouse_highlight);
    assert!(!unsupported_highlight.mouse_reporting());

    let highlight_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::MouseReport))
            .unwrap_err();
    assert!(
        highlight_error.starts_with("WHEEL_MODE_CHANGED:"),
        "DEC 1001 must not be aliased to another Ghostty tracking mode: {highlight_error}"
    );
}
