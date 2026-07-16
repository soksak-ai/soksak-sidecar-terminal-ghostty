//! 엔진 격리 좌석 — libghostty-vt 를 만지는 유일한 모듈. 미러(복원 직렬화기)는 여기가
//! 내놓는 엔진-중립 뷰(스칼라 상태 + [`GridCell`] 행 읽기)만 쓴다. 이 좌석이 만든 페인트는
//! 계약 kit 의 판정자(다른 엔진의 렌더러)가 채점한다 — 두 엔진의 잠복 버그가 서로 뒤에 못
//! 숨는다.
//!
//! 엔진-중립 타입([`ColorSnap`]·[`ModeSnap`]·[`GridCell`])은 직렬화기가 그리드를 읽는 창이다.
//! [`Engine`] 만 ghostty 이고, C 타입은 이 파일의 [`ffi`] 모듈 밖으로 나가지 않는다.
//!
//! ghostty 가 흡수한 엔진 차이(엔진-중립 면의 시그니처는 계약이 고정한다):
//!   - 응답 포획: ghostty 는 질의 응답을 실제로 만들 수 있다. 콜백을 안 걸면 무음 폐기가
//!     기본이지만, 그러면 삼킴이 관측되지 않는다. 그래서 응답 경로마다 **계수-후-폐기**
//!     콜백을 세운다: DSR·DECRQM·OSC 색 질의·ENQ 응답은 `write_pty` 로, DA1/DA2/DA3 는
//!     `device_attributes` 로 흘러온다. 둘 다 계수만 하고 바이트를 버린다(device_attributes
//!     는 false 반환 = 응답 없음). PTY 로 되쓰는 경로는 이 좌석에 존재하지 않는다.
//!   - private mode 읽기: ghostty 는 모든 모드에 `mode_get` 이 있다 — 관찰로 재구성할 것이
//!     없다. 기본 켜짐 모드(wraparound 7·cursor_visible 25·alternate_scroll 1007)가 계약의 선언과
//!     같아, 직렬화기의 "기본과 다른 것만 내보낸다" 규칙이 두 엔진에서 같은 뜻이다.
//!   - 스크롤백 읽기: history 좌표계의 랜덤 접근(grid_ref)이라 읽기가 순수하다 — 읽기 커서를
//!     옮겼다 되돌릴 필요가 없어 그리드 읽기가 `&self` 다.
//!   - 그리드 폭: wide 문자를 본체 셀 + 스페이서(tail/head)로 담는다(계약 정규형과 동형). 스페이서는
//!     문자를 담지 않는다.
//!   - 셀 배경: 텍스트 없는 배경-전용 셀은 content_tag 가 배경색을 들고, 그 외 셀은 style 이
//!     든다(엔진의 저장 최적화). 두 자리를 다 읽어야 배경이 맞는다. 팔레트 색은 인덱스를 그대로
//!     읽는다 — RGB 로 해소하면 직렬화가 truecolor 로 나가 원본의 Named/Indexed 와 어긋난다.
//!   - charset: DEC Special Graphics 번역이 print 시점에 끝나 셀 codepoint 가 이미 박스
//!     글리프(─│┌┐└┘)다 — 좌석이 charset 을 따로 만질 것이 없다.

use std::ffi::c_void;
use std::os::raw::c_int;

/// 엔진이 유지하는 스크롤백 행 수. 바이트 충실 복원의 바닥 — 전체 의미 이력은
/// command_blocks(app.data)가 소유하고, 이 수치는 화면 재현용 창이다.
pub const MIRROR_SCROLLBACK_LINES: usize = 1000;

/// 엔진에 주는 스크롤백 예산(바이트). ghostty 의 스크롤백 한계는 **행이 아니라 바이트**이고,
/// 한계를 넘으면 가장 오래된 **페이지를 통째로** 버린다 — 남는 행 수가 한 페이지만큼 뚝
/// 떨어진다. 그래서 예산을 [`MIRROR_SCROLLBACK_LINES`] 로 주면 가지치기 직후 보존 행이 창보다
/// 적어져 복원 화면이 원본보다 짧아진다. 예산은 창을 페이지 하나 이상 넉넉히 덮도록 바이트로
/// 잡고, 창은 좌석이 [`Engine::history_size`] 에서 잘라 계약대로 최신 N 행만 노출한다.
/// (스타일·그래핌이 촘촘한 행일수록 페이지에 적게 들어간다 — 무거운 내용 기준으로 잡는다.
/// `engine_retains_the_whole_window_under_heavy_content` 가 이 값을 지킨다.)
const SCROLLBACK_BUDGET_BYTES: usize = 8 * 1024 * 1024;

// ── 엔진-중립 스냅샷 타입(계약의 비교 통화 — 두 엔진 유닛 공용) ──────────────

/// 색 스냅샷 — 엔진 타입을 밖으로 새지 않게 자체 표현으로 고정한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorSnap {
    Default,
    Named(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// 복원 대상 private mode 집합의 스냅샷(rehydrate 가 재현해야 하는 전부).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModeSnap {
    pub bracketed_paste: bool,
    pub app_cursor: bool,
    pub app_keypad: bool,
    pub mouse_click: bool,
    pub mouse_drag: bool,
    pub mouse_motion: bool,
    pub sgr_mouse: bool,
    pub utf8_mouse: bool,
    pub focus_in_out: bool,
    pub alternate_scroll: bool,
    pub show_cursor: bool,
    pub line_wrap: bool,
    pub insert: bool,
}

/// 직렬화기가 읽는 엔진-중립 셀 — 직렬화에 필요한 것을 다 담는다(spacer·wrapline·zerowidth
/// 포함). 이 타입 하나가 직렬화기의 그리드 읽기 단일 창이다 — 엔진 세부는 이 파일 밖으로
/// 나가지 않는다.
pub struct GridCell {
    pub ch: char,
    pub fg: ColorSnap,
    pub bg: ColorSnap,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub strikeout: bool,
    pub hidden: bool,
    /// wide 문자 본체(2칸 점유의 첫 칸).
    pub wide: bool,
    /// wide 문자 스페이서(본체 뒤 칸) — 직렬화기가 건너뛴다.
    pub spacer: bool,
    /// WRAPLINE — 마지막 칸에서만 의미: 이 행이 자연 개행(wrap)으로 이어진다.
    pub wrapline: bool,
    /// 결합 문자(zero-width) 후속.
    pub zerowidth: Vec<char>,
}

fn blank_cell() -> GridCell {
    GridCell {
        ch: ' ',
        fg: ColorSnap::Default,
        bg: ColorSnap::Default,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        inverse: false,
        strikeout: false,
        hidden: false,
        wide: false,
        spacer: false,
        wrapline: false,
        zerowidth: Vec::new(),
    }
}

// ── ffi — libghostty-vt C-ABI(정본 = vendor/ghostty include/ghostty/vt/*.h) ────
// C 타입·상수는 여기 갇혀 있다. 링크는 build.rs 가 정적 libghostty-vt.a 로 건다.

#[allow(non_camel_case_types)]
mod ffi {
    use std::ffi::c_void;
    use std::os::raw::c_int;

    /// 불투명 터미널 핸들.
    pub type Terminal = *mut c_void;
    /// 불투명 셀·행 값(u64).
    pub type Cell = u64;
    pub type Row = u64;

    // GhosttyResult
    pub const SUCCESS: c_int = 0;
    pub const OUT_OF_SPACE: c_int = -3;

    // GhosttyTerminalOption
    pub const OPT_USERDATA: c_int = 0;
    pub const OPT_WRITE_PTY: c_int = 1;
    pub const OPT_DEVICE_ATTRIBUTES: c_int = 8;

    // GhosttyTerminalData
    pub const DATA_CURSOR_X: c_int = 3;
    pub const DATA_CURSOR_Y: c_int = 4;
    pub const DATA_ACTIVE_SCREEN: c_int = 6;
    pub const DATA_SCROLLBACK_ROWS: c_int = 15;

    // GhosttyTerminalScreen
    pub const SCREEN_ALTERNATE: c_int = 1;

    // GhosttyPointTag
    pub const POINT_TAG_ACTIVE: c_int = 0;
    pub const POINT_TAG_HISTORY: c_int = 3;

    // GhosttyCellData
    pub const CELL_DATA_CODEPOINT: c_int = 1;
    pub const CELL_DATA_CONTENT_TAG: c_int = 2;
    pub const CELL_DATA_WIDE: c_int = 3;
    pub const CELL_DATA_COLOR_PALETTE: c_int = 10;
    pub const CELL_DATA_COLOR_RGB: c_int = 11;

    // GhosttyCellContentTag
    pub const CONTENT_CODEPOINT_GRAPHEME: c_int = 1;
    pub const CONTENT_BG_COLOR_PALETTE: c_int = 2;
    pub const CONTENT_BG_COLOR_RGB: c_int = 3;

    // GhosttyCellWide
    pub const WIDE_NARROW: c_int = 0;
    pub const WIDE_WIDE: c_int = 1;
    pub const WIDE_SPACER_TAIL: c_int = 2;
    pub const WIDE_SPACER_HEAD: c_int = 3;

    // GhosttyRowData
    pub const ROW_DATA_WRAP: c_int = 1;

    // GhosttyStyleColorTag
    pub const STYLE_COLOR_PALETTE: c_int = 1;
    pub const STYLE_COLOR_RGB: c_int = 2;

    // GhosttySgrUnderline
    pub const SGR_UNDERLINE_NONE: c_int = 0;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct ColorRgb {
        pub r: u8,
        pub g: u8,
        pub b: u8,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct TerminalOptions {
        pub cols: u16,
        pub rows: u16,
        pub max_scrollback: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct PointCoordinate {
        pub x: u16,
        pub y: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union PointValue {
        pub coordinate: PointCoordinate,
        pub _padding: [u64; 2],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Point {
        pub tag: c_int,
        pub value: PointValue,
    }

    /// sized struct — 호출자가 `size` 를 채운다.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct GridRef {
        pub size: usize,
        pub node: *mut c_void,
        pub x: u16,
        pub y: u16,
    }

    impl GridRef {
        pub fn sized() -> Self {
            GridRef {
                size: std::mem::size_of::<GridRef>(),
                node: std::ptr::null_mut(),
                x: 0,
                y: 0,
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union StyleColorValue {
        pub palette: u8,
        pub rgb: ColorRgb,
        pub _padding: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct StyleColor {
        pub tag: c_int,
        pub value: StyleColorValue,
    }

    /// sized struct — 호출자가 `size` 를 채운다.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Style {
        pub size: usize,
        pub fg_color: StyleColor,
        pub bg_color: StyleColor,
        pub underline_color: StyleColor,
        pub bold: bool,
        pub italic: bool,
        pub faint: bool,
        pub blink: bool,
        pub inverse: bool,
        pub invisible: bool,
        pub strikethrough: bool,
        pub overline: bool,
        pub underline: c_int,
    }

    impl Style {
        pub fn sized() -> Self {
            // 전 필드 0 = 기본 스타일(색 none·플래그 off). size 만 채워 ABI 판을 알린다.
            let mut s: Style = unsafe { std::mem::zeroed() };
            s.size = std::mem::size_of::<Style>();
            s
        }
    }

    /// write_pty 효과 — 터미널이 PTY 에 되쓰려는 응답 바이트.
    pub type WritePtyFn = extern "C" fn(Terminal, *mut c_void, *const u8, usize);
    /// device_attributes 효과 — DA1/DA2/DA3 질의. false 반환 = 응답 없음.
    pub type DeviceAttributesFn = extern "C" fn(Terminal, *mut c_void, *mut c_void) -> bool;

    extern "C" {
        pub fn ghostty_terminal_new(
            allocator: *const c_void,
            terminal: *mut Terminal,
            options: TerminalOptions,
        ) -> c_int;
        pub fn ghostty_terminal_free(terminal: Terminal);
        pub fn ghostty_terminal_resize(
            terminal: Terminal,
            cols: u16,
            rows: u16,
            cell_width_px: u32,
            cell_height_px: u32,
        ) -> c_int;
        pub fn ghostty_terminal_set(
            terminal: Terminal,
            option: c_int,
            value: *const c_void,
        ) -> c_int;
        pub fn ghostty_terminal_vt_write(terminal: Terminal, data: *const u8, len: usize);
        pub fn ghostty_terminal_mode_get(terminal: Terminal, mode: u16, out: *mut bool) -> c_int;
        pub fn ghostty_terminal_get(terminal: Terminal, data: c_int, out: *mut c_void) -> c_int;
        pub fn ghostty_terminal_grid_ref(
            terminal: Terminal,
            point: Point,
            out_ref: *mut GridRef,
        ) -> c_int;
        pub fn ghostty_grid_ref_cell(gref: *const GridRef, out_cell: *mut Cell) -> c_int;
        pub fn ghostty_grid_ref_row(gref: *const GridRef, out_row: *mut Row) -> c_int;
        pub fn ghostty_grid_ref_style(gref: *const GridRef, out_style: *mut Style) -> c_int;
        pub fn ghostty_grid_ref_graphemes(
            gref: *const GridRef,
            buf: *mut u32,
            buf_len: usize,
            out_len: *mut usize,
        ) -> c_int;
        pub fn ghostty_cell_get(cell: Cell, data: c_int, out: *mut c_void) -> c_int;
        pub fn ghostty_row_get(row: Row, data: c_int, out: *mut c_void) -> c_int;
    }
}

// 모드 식별자 패킹(modes.h): 하위 15비트 = 값, 최상위 비트 = ANSI 플래그(0 = DEC private).
fn dec_mode(value: u16) -> u16 {
    value & 0x7FFF
}
fn ansi_mode(value: u16) -> u16 {
    (value & 0x7FFF) | (1 << 15)
}

// ── 삼킨 응답 계수 — 콜백의 userdata ─────────────────────────────────────────
// 엔진은 응답을 만들 수 있으므로, 응답 경로를 가로채 계수하고 바이트를 버린다. 미러가
// PTY 에 되쓰는 경로는 없다 — 질의의 단일 응답자는 프론트 터미널 하나다.

struct Counters {
    suppressed: u64,
}

// userdata 포인터에서 계수기를 되찾아 1 올린다. 콜백은 vt_write 안에서 동기 호출된다.
unsafe fn bump(userdata: *mut c_void) {
    if let Some(c) = (userdata as *mut Counters).as_mut() {
        c.suppressed = c.suppressed.saturating_add(1);
    }
}

// DSR·DECRQM·OSC 색 질의·ENQ 응답이 여기로 온다. 바이트는 버린다(복사조차 하지 않는다 —
// 빌려온 버퍼는 콜백 밖에서 무효다).
extern "C" fn cb_write_pty(_t: ffi::Terminal, userdata: *mut c_void, _data: *const u8, _len: usize) {
    unsafe { bump(userdata) };
}

// DA1/DA2/DA3 질의. false = 응답을 만들지 않는다(무음 무시) — 계수만 남긴다.
extern "C" fn cb_device_attributes(
    _t: ffi::Terminal,
    userdata: *mut c_void,
    _out: *mut c_void,
) -> bool {
    unsafe { bump(userdata) };
    false
}

// ── Engine — 유일한 libghostty-vt 좌석 ───────────────────────────────────────

/// 바이트를 실제 렌더해 화면 상태를 유지하는 헤드리스 VT 엔진(ghostty). 미러(복원 로직)가
/// 쓰는 유일한 엔진 면이며, "이 바이트를 먹은 터미널이 PTY 에 무엇을 되쓰려 했는가"의
/// 프로브(`suppressed_replies`)이기도 하다.
pub struct Engine {
    term: ffi::Terminal,
    // 콜백 userdata — 힙에 고정된 주소를 터미널에 넘겼다. Engine 이 움직여도 이 주소는
    // 그대로다(Box). 터미널보다 오래 살아야 하므로 Drop 순서가 중요하다(아래 Drop 참조).
    counters: Box<Counters>,
    cols: u16,
    rows: u16,
}

// 터미널은 이 Engine 이 단독 소유하며 공유되지 않는다(서비스는 세션마다 미러 하나를 잠금
// 뒤에 둔다). 소유권째 스레드로 옮기는 것은 안전하다 — 동시 접근은 일어나지 않는다.
// Sync 는 주장하지 않는다(엔진은 동시 &-접근에 안전하지 않다).
unsafe impl Send for Engine {}

impl Engine {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let mut term: ffi::Terminal = std::ptr::null_mut();
        let opts =
            ffi::TerminalOptions { cols, rows, max_scrollback: SCROLLBACK_BUDGET_BYTES };
        // allocator = NULL → 엔진 기본 할당자.
        let r = unsafe { ffi::ghostty_terminal_new(std::ptr::null(), &mut term, opts) };
        assert!(
            r == ffi::SUCCESS && !term.is_null(),
            "ghostty_terminal_new failed (result {r})"
        );

        let mut counters = Box::new(Counters { suppressed: 0 });
        let userdata: *mut c_void = (&mut *counters as *mut Counters).cast();
        let write_pty: ffi::WritePtyFn = cb_write_pty;
        let device_attributes: ffi::DeviceAttributesFn = cb_device_attributes;
        unsafe {
            // 콜백·userdata 는 값이 곧 포인터다(터미널이 그대로 보관).
            ffi::ghostty_terminal_set(term, ffi::OPT_USERDATA, userdata);
            ffi::ghostty_terminal_set(term, ffi::OPT_WRITE_PTY, write_pty as *const c_void);
            ffi::ghostty_terminal_set(
                term,
                ffi::OPT_DEVICE_ATTRIBUTES,
                device_attributes as *const c_void,
            );
        }

        Engine { term, counters, cols, rows }
    }

    /// 세션 출력 바이트 소비. 응답 요구 시퀀스는 콜백에서 계수되고 버려진다 — 나가는 바이트는
    /// 없다. 엔진은 악성 입력을 가정하므로 이 호출은 실패하지 않는다.
    pub fn feed(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        unsafe { ffi::ghostty_terminal_vt_write(self.term, bytes.as_ptr(), bytes.len()) };
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        // 헤드리스라 픽셀 격자는 없다(px 는 이미지 프로토콜·size report 전용).
        unsafe { ffi::ghostty_terminal_resize(self.term, self.cols, self.rows, 0, 0) };
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn alt_active(&self) -> bool {
        let mut screen: c_int = 0;
        let r = unsafe {
            ffi::ghostty_terminal_get(
                self.term,
                ffi::DATA_ACTIVE_SCREEN,
                (&mut screen as *mut c_int).cast(),
            )
        };
        r == ffi::SUCCESS && screen == ffi::SCREEN_ALTERNATE
    }

    /// 커서 위치(화면 기준 0-base row, col).
    pub fn cursor(&self) -> (usize, usize) {
        let mut x: u16 = 0;
        let mut y: u16 = 0;
        unsafe {
            ffi::ghostty_terminal_get(self.term, ffi::DATA_CURSOR_X, (&mut x as *mut u16).cast());
            ffi::ghostty_terminal_get(self.term, ffi::DATA_CURSOR_Y, (&mut y as *mut u16).cast());
        }
        (y as usize, x as usize)
    }

    /// 복원 창의 스크롤백 행 수 — 계약이 재현하는 창은 최신 [`MIRROR_SCROLLBACK_LINES`] 행이다.
    /// 엔진은 예산이 허락하는 만큼 그보다 더 들고 있을 수 있으나(바이트 예산·페이지 단위 보존),
    /// 좌석은 창을 넘겨 내보내지 않는다 — 직렬화기가 페인트에 싣는 행 수의 상한이다.
    pub fn history_size(&self) -> usize {
        self.retained_rows().min(MIRROR_SCROLLBACK_LINES)
    }

    /// 엔진이 실제로 들고 있는 스크롤백 행 수(창보다 클 수 있다). 좌표 계산의 기준이다 —
    /// history 좌표는 실제 보존분의 0(최고참)부터 매겨지므로, 창으로 자른 값으로 색인하면
    /// 엉뚱한 행을 읽는다.
    fn retained_rows(&self) -> usize {
        let mut n: usize = 0;
        let r = unsafe {
            ffi::ghostty_terminal_get(
                self.term,
                ffi::DATA_SCROLLBACK_ROWS,
                (&mut n as *mut usize).cast(),
            )
        };
        if r == ffi::SUCCESS {
            n
        } else {
            0
        }
    }

    pub fn modes(&self) -> ModeSnap {
        ModeSnap {
            bracketed_paste: self.mode(dec_mode(2004)),
            app_cursor: self.mode(dec_mode(1)),
            // DECKPAM(ESC =)이 세우는 모드가 keypad_keys(66)다.
            app_keypad: self.mode(dec_mode(66)),
            mouse_click: self.mode(dec_mode(1000)),
            mouse_drag: self.mode(dec_mode(1002)),
            mouse_motion: self.mode(dec_mode(1003)),
            sgr_mouse: self.mode(dec_mode(1006)),
            utf8_mouse: self.mode(dec_mode(1005)),
            focus_in_out: self.mode(dec_mode(1004)),
            alternate_scroll: self.mode(dec_mode(1007)),
            show_cursor: self.mode(dec_mode(25)),
            line_wrap: self.mode(dec_mode(7)),
            insert: self.mode(ansi_mode(4)),
        }
    }

    fn mode(&self, packed: u16) -> bool {
        let mut v = false;
        let r = unsafe { ffi::ghostty_terminal_mode_get(self.term, packed, &mut v) };
        r == ffi::SUCCESS && v
    }

    /// 미러가 관찰한, 삼킨 응답 요구 수(DA1/DA2/DA3·DSR·DECRQM·OSC 질의). 엔진은 응답을
    /// 만들 수 있지만 좌석이 전부 계수-후-폐기했다 — 나간 바이트는 0 이다.
    pub fn suppressed_replies(&self) -> u64 {
        self.counters.suppressed
    }

    /// 한 행(line index; 0..rows = 보이는 화면, 음수 = 스크롤백)을 엔진-중립 셀 벡터로
    /// 읽는다. 길이는 항상 `cols` — spacer 포함(직렬화기가 skip 판정을 소유한다). 스크롤백은
    /// history 좌표계로 직접 조회한다(읽기가 순수 — 상태를 건드리지 않는다).
    pub fn line_cells(&self, line: i32) -> Vec<GridCell> {
        let cols = self.cols;
        let (tag, y) = if line >= 0 {
            (ffi::POINT_TAG_ACTIVE, line as u32)
        } else {
            // history 좌표는 실제 보존분의 0(최고참)부터 매겨진다 — 창(history_size)이 아니라
            // 보존 행 수로 색인해야 line -1 이 최신을 가리킨다.
            let retained = self.retained_rows() as i32;
            let y = retained + line;
            if y < 0 {
                return (0..cols).map(|_| blank_cell()).collect();
            }
            (ffi::POINT_TAG_HISTORY, y as u32)
        };

        let mut out: Vec<GridCell> = Vec::with_capacity(cols as usize);
        let mut wrapped = false;
        for col in 0..cols {
            match self.grid_ref(tag, col, y) {
                Some(gref) => {
                    // 행 속성(wrap)은 행 전체의 것이다 — 첫 칸에서 한 번만 읽는다.
                    if col == 0 {
                        wrapped = row_wrapped(&gref);
                    }
                    out.push(cell_of(&gref));
                }
                None => out.push(blank_cell()),
            }
        }
        if wrapped {
            if let Some(last) = out.last_mut() {
                last.wrapline = true;
            }
        }
        out
    }

    // 좌표를 그리드 참조로 해석한다. 참조는 다음 변경 호출 전까지만 유효하므로 곧바로 읽는다.
    fn grid_ref(&self, tag: c_int, x: u16, y: u32) -> Option<ffi::GridRef> {
        let point = ffi::Point {
            tag,
            value: ffi::PointValue { coordinate: ffi::PointCoordinate { x, y } },
        };
        let mut gref = ffi::GridRef::sized();
        let r = unsafe { ffi::ghostty_terminal_grid_ref(self.term, point, &mut gref) };
        if r != ffi::SUCCESS || gref.node.is_null() {
            return None;
        }
        Some(gref)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // 터미널을 먼저 놓는다 — 그 뒤엔 콜백이 불릴 수 없으므로 counters 해제가 안전하다
        // (필드는 이 함수가 끝난 뒤 선언 순서로 떨어진다).
        unsafe { ffi::ghostty_terminal_free(self.term) };
        self.term = std::ptr::null_mut();
    }
}

// ── 그리드 읽기 — C 셀/행/스타일 → 엔진-중립 GridCell ─────────────────────────

fn row_wrapped(gref: &ffi::GridRef) -> bool {
    let mut row: ffi::Row = 0;
    if unsafe { ffi::ghostty_grid_ref_row(gref, &mut row) } != ffi::SUCCESS {
        return false;
    }
    let mut wrap = false;
    let r =
        unsafe { ffi::ghostty_row_get(row, ffi::ROW_DATA_WRAP, (&mut wrap as *mut bool).cast()) };
    r == ffi::SUCCESS && wrap
}

fn cell_of(gref: &ffi::GridRef) -> GridCell {
    let mut cell: ffi::Cell = 0;
    if unsafe { ffi::ghostty_grid_ref_cell(gref, &mut cell) } != ffi::SUCCESS {
        return blank_cell();
    }

    // wide 스페이서는 본체 뒤의 점유 칸이다 — 문자를 담지 않는다.
    let mut wide_tag: c_int = ffi::WIDE_NARROW;
    unsafe {
        ffi::ghostty_cell_get(cell, ffi::CELL_DATA_WIDE, (&mut wide_tag as *mut c_int).cast())
    };
    if wide_tag == ffi::WIDE_SPACER_TAIL || wide_tag == ffi::WIDE_SPACER_HEAD {
        return GridCell { spacer: true, ..blank_cell() };
    }

    let mut content: c_int = 0;
    unsafe {
        ffi::ghostty_cell_get(cell, ffi::CELL_DATA_CONTENT_TAG, (&mut content as *mut c_int).cast())
    };

    let mut style = ffi::Style::sized();
    unsafe { ffi::ghostty_grid_ref_style(gref, &mut style) };

    // 배경-전용 셀은 content 가 색을 든다(스타일이 아니라). 그 외는 스타일이 든다.
    let (ch, bg) = match content {
        ffi::CONTENT_BG_COLOR_PALETTE => {
            let mut idx: u8 = 0;
            unsafe {
                ffi::ghostty_cell_get(
                    cell,
                    ffi::CELL_DATA_COLOR_PALETTE,
                    (&mut idx as *mut u8).cast(),
                )
            };
            (' ', palette_snap(idx))
        }
        ffi::CONTENT_BG_COLOR_RGB => {
            let mut rgb = ffi::ColorRgb { r: 0, g: 0, b: 0 };
            unsafe {
                ffi::ghostty_cell_get(
                    cell,
                    ffi::CELL_DATA_COLOR_RGB,
                    (&mut rgb as *mut ffi::ColorRgb).cast(),
                )
            };
            (' ', ColorSnap::Rgb(rgb.r, rgb.g, rgb.b))
        }
        _ => {
            let mut cp: u32 = 0;
            unsafe {
                ffi::ghostty_cell_get(cell, ffi::CELL_DATA_CODEPOINT, (&mut cp as *mut u32).cast())
            };
            // codepoint 0 = 빈 칸.
            let ch = if cp == 0 { ' ' } else { char::from_u32(cp).unwrap_or(' ') };
            (ch, style_color_snap(&style.bg_color))
        }
    };

    // 결합 문자는 클러스터를 든 셀에만 있다 — 그 외 셀에 FFI 왕복을 낭비하지 않는다.
    let zerowidth =
        if content == ffi::CONTENT_CODEPOINT_GRAPHEME { graphemes_of(gref) } else { Vec::new() };

    GridCell {
        ch,
        fg: style_color_snap(&style.fg_color),
        bg,
        bold: style.bold,
        dim: style.faint,
        italic: style.italic,
        underline: style.underline != ffi::SGR_UNDERLINE_NONE,
        inverse: style.inverse,
        strikeout: style.strikethrough,
        hidden: style.invisible,
        wide: wide_tag == ffi::WIDE_WIDE,
        spacer: false,
        wrapline: false,
        zerowidth,
    }
}

// 그래핌 클러스터의 결합 문자들(본체 codepoint 뒤). 대부분의 클러스터는 짧으므로 스택 버퍼로
// 시도하고, 넘치면 필요한 크기를 받아 다시 읽는다.
fn graphemes_of(gref: &ffi::GridRef) -> Vec<char> {
    let mut buf = [0u32; 8];
    let mut len: usize = 0;
    let r = unsafe {
        ffi::ghostty_grid_ref_graphemes(gref, buf.as_mut_ptr(), buf.len(), &mut len)
    };
    let cps: Vec<u32> = match r {
        ffi::SUCCESS => buf[..len.min(buf.len())].to_vec(),
        ffi::OUT_OF_SPACE => {
            let mut heap = vec![0u32; len];
            let mut got: usize = 0;
            let r2 = unsafe {
                ffi::ghostty_grid_ref_graphemes(gref, heap.as_mut_ptr(), heap.len(), &mut got)
            };
            if r2 != ffi::SUCCESS {
                return Vec::new();
            }
            heap.truncate(got);
            heap
        }
        _ => return Vec::new(),
    };
    // [0] 은 본체 codepoint — 결합 문자는 그 뒤다.
    cps.iter().skip(1).filter_map(|c| char::from_u32(*c)).collect()
}

// 스타일 색 → 엔진-중립 ColorSnap. 팔레트는 인덱스를 그대로 옮긴다(RGB 로 해소하면 직렬화가
// truecolor 로 나가 원본의 Named/Indexed 와 어긋난다).
fn style_color_snap(c: &ffi::StyleColor) -> ColorSnap {
    match c.tag {
        ffi::STYLE_COLOR_PALETTE => palette_snap(unsafe { c.value.palette }),
        ffi::STYLE_COLOR_RGB => {
            let rgb = unsafe { c.value.rgb };
            ColorSnap::Rgb(rgb.r, rgb.g, rgb.b)
        }
        _ => ColorSnap::Default,
    }
}

// 팔레트 0..16 은 Named(기본/브라이트 SGR 로 왕복), 16..256 은 Indexed(38;5;N) — 판정자의
// Named/Indexed 매핑과 동형.
fn palette_snap(i: u8) -> ColorSnap {
    if i < 16 {
        ColorSnap::Named(i)
    } else {
        ColorSnap::Indexed(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 엔진은 질의에 응답을 만들 수 있다 — 좌석이 응답 경로(write_pty·device_attributes)를
    // 계수-후-폐기로 막았다. 각 질의를 신선한 엔진에 먹여 계수가 오르는지 곧바로 단언한다.
    #[test]
    fn reply_producing_queries_are_counted_but_never_answered() {
        for q in [&b"\x1b[c"[..], b"\x1b[>c", b"\x1b[6n"] {
            let mut e = Engine::new(80, 24);
            e.feed(q);
            assert!(
                e.suppressed_replies() > 0,
                "feed must count the swallowed query {q:?}"
            );
        }
    }

    // OSC 색 질의(`OSC 10/11 ; ?`)는 이 미러에서 응답 자체가 없다 — 미러는 표시 장치가 아니라
    // 테마를 갖지 않고, 엔진은 설정된 색이 없으면 보고할 것이 없어 응답을 만들지 않는다(삼킬
    // 바이트도 없다). 불변식은 그대로다: 나가는 바이트 0. 라이브 세션의 단일 응답자는 색을 실제로
    // 가진 프론트 터미널이다.
    #[test]
    fn color_queries_have_no_reply_to_swallow_on_an_unthemed_mirror() {
        let mut e = Engine::new(80, 24);
        e.feed(b"\x1b]11;?\x07");
        assert_eq!(
            e.suppressed_replies(),
            0,
            "no color is configured — the engine produces no reply to suppress"
        );
    }

    // private mode 는 전부 네이티브 getter 로 읽는다 — 기본값과 다른 것을 세운 뒤 확인.
    #[test]
    fn private_modes_are_read_natively() {
        let mut e = Engine::new(80, 24);
        e.feed(b"\x1b[?1004h\x1b[?1007l\x1b[?7l\x1b[4h");
        let m = e.modes();
        assert!(m.focus_in_out, "focus(1004) set");
        assert!(!m.alternate_scroll, "alt-scroll(1007) cleared");
        assert!(!m.line_wrap, "auto-wrap(7) cleared");
        assert!(m.insert, "insert(4) set");
    }

    // 신선한 터미널의 기본 켜짐 모드가 판정자와 같아야 직렬화기의 "기본과 다른 것만 내보낸다"
    // 규칙이 성립한다.
    #[test]
    fn fresh_defaults_match_the_contract() {
        let e = Engine::new(80, 24);
        let m = e.modes();
        assert!(m.line_wrap, "auto-wrap(7) defaults on");
        assert!(m.show_cursor, "cursor(25) defaults on");
        assert!(m.alternate_scroll, "alt-scroll(1007) defaults on");
        assert!(!m.bracketed_paste && !m.app_cursor && !m.insert, "the rest default off");
    }

    // DEC Special Graphics 는 print 시점에 번역돼 셀 codepoint 가 이미 박스 글리프다 —
    // 좌석은 번역된 글자를 그대로 읽는다.
    #[test]
    fn dec_special_graphics_lands_translated_in_the_grid() {
        let mut e = Engine::new(80, 24);
        e.feed(b"\x1b(0lqk\x1b(B");
        let row: String = e.line_cells(0).iter().map(|c| c.ch).collect();
        assert!(
            row.starts_with("┌─┐"),
            "charset translation must land in the grid (got {row:?})"
        );
    }

    // 엔진의 스크롤백 한계는 바이트 예산이고 가지치기는 페이지 통째로다 — 예산이 얕으면 무거운
    // 내용(wide + 셀마다 다른 트루컬러)에서 한 페이지가 떨어져 나가 보존 행이 창 밑으로 꺼진다.
    // 그러면 복원 화면이 원본보다 짧아진다(계약 위반). 예산이 창 전체를 실제로 뒷받침하는지
    // 무거운 내용으로 못박는다 — 픽스처 ④가 잡아낸 그 조건이다.
    #[test]
    fn engine_retains_the_whole_window_under_heavy_content() {
        let mut e = Engine::new(80, 24);
        // 셀마다 색이 다른 wide 문자 40개 = 80칸을 꽉 채운 행(스타일·그래핌이 페이지를 빨리 먹는다).
        for i in 0..(MIRROR_SCROLLBACK_LINES + 200) {
            let mut row = String::new();
            for j in 0..40 {
                let r = 100 + ((i * 7 + j * 13) % 156);
                let g = 100 + ((i * 11 + j * 3) % 156);
                let b = 100 + ((i * 5 + j * 17) % 156);
                row.push_str(&format!("\x1b[0;1;38;2;{r};{g};{b}m가"));
            }
            row.push_str("\x1b[0m\r\n");
            e.feed(row.as_bytes());
        }
        assert!(
            e.retained_rows() >= MIRROR_SCROLLBACK_LINES,
            "the scrollback budget must back the whole window (retained {}, window {})",
            e.retained_rows(),
            MIRROR_SCROLLBACK_LINES
        );
        assert_eq!(
            e.history_size(),
            MIRROR_SCROLLBACK_LINES,
            "the seat reports exactly the contract window"
        );
    }

    // 스크롤백은 history 좌표로 직접 읽는다 — line -1 이 최신(보이는 화면 바로 위), line -H 가
    // 최고참. 3행 화면에 6줄(끝마다 개행)을 먹이면 화면은 [L5, L6, ""], 이력은 [L1..L4] 다.
    #[test]
    fn scrollback_indexes_newest_at_minus_one() {
        let mut e = Engine::new(80, 3);
        for i in 1..=6 {
            e.feed(format!("L{i}\r\n").as_bytes());
        }
        assert_eq!(e.history_size(), 4, "four rows scrolled into history");
        let newest: String = e.line_cells(-1).iter().map(|c| c.ch).collect();
        let oldest: String = e.line_cells(-4).iter().map(|c| c.ch).collect();
        assert!(newest.starts_with("L4"), "line -1 is the newest history row (got {newest:?})");
        assert!(oldest.starts_with("L1"), "line -4 is the oldest history row (got {oldest:?})");
    }
}
