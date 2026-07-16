//! libghostty-vt 정적 링크. 엔진은 C 라이브러리라 cargo 의존성이 아니라 링크 대상이다 —
//! 그 산출물을 어디서 찾아 어떻게 거는지가 이 파일의 전부다.
//!
//! 경로는 추측하지 않는다: 선언(`SOKSAK_GHOSTTY_VT_LIB`)이 우선이고, 없으면 벤더 규약
//! (`<unit>/../../vendor/ghostty/zig-out/lib`)으로 발견한다. 라이브러리가 없으면 빌드를
//! 조용히 통과시키지 않고, 만드는 법을 적어 실패한다(무음 금지).
//!
//! 엔진의 lib 디렉토리에는 정적 아카이브와 dylib 이 함께 있고, macOS 링커는 같은 이름이면
//! dylib 을 먼저 집는다 — 그러면 실행 시 `@rpath/libghostty-vt.dylib` 를 찾다 죽는다. 그래서
//! 아카이브만 OUT_DIR 에 스테이징해 그 디렉토리를 링크 검색 경로로 준다(모호성 제거). 사이드카
//! 바이너리는 엔진을 안고 다녀야 한다 — 런타임에 찾아야 할 공유 라이브러리를 만들지 않는다.
//!
//! 아카이브 이름은 플랫폼마다 다르다: Windows 는 정적 라이브러리를 `ghostty-vt-static.lib`
//! 로 낸다(DLL import 라이브러리 `ghostty-vt.lib` 와의 충돌을 피하려고 ghostty 가 붙인 이름,
//! build.zig 참조). 나머지 플랫폼은 `libghostty-vt.a`. 링크 이름도 그에 맞춘다.
//!
//! 산출물을 만드는 법은 README 의 빌드 요구사항(zig 판·ghostty 커밋 핀)이 정본이다.

use std::path::PathBuf;

fn main() {
    // 타깃(호스트가 아니라 빌드 대상) OS 로 아카이브·링크 이름을 고른다 — 크로스 빌드에도 옳다.
    let target_windows =
        std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let archive_name = if target_windows {
        "ghostty-vt-static.lib"
    } else {
        "libghostty-vt.a"
    };
    let link_name = if target_windows { "ghostty-vt-static" } else { "ghostty-vt" };

    let vendor_lib_dir = match std::env::var("SOKSAK_GHOSTTY_VT_LIB") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => {
            let manifest = PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").expect("cargo supplies CARGO_MANIFEST_DIR"),
            );
            manifest.join("../../vendor/ghostty/zig-out/lib")
        }
    };

    let archive = vendor_lib_dir.join(archive_name);
    if !archive.is_file() {
        panic!(
            "{archive_name} not found at {}\n\
             Build the vendored engine first (see README, Build requirements):\n\
             \x20 cd <vendor>/ghostty && <zig> build -Demit-lib-vt=true -Doptimize=ReleaseFast\n\
             Or point SOKSAK_GHOSTTY_VT_LIB at a directory that holds {archive_name}.",
            archive.display()
        );
    }

    // 아카이브만 있는 검색 경로 — 옆의 dylib(또는 Windows 의 import lib)이 링커에 잡히지 않는다.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("cargo supplies OUT_DIR"));
    let staged = out_dir.join(archive_name);
    std::fs::copy(&archive, &staged)
        .unwrap_or_else(|e| panic!("staging {} into OUT_DIR failed: {e}", archive.display()));

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static={link_name}");
    println!("cargo:rerun-if-changed={}", archive.display());
    println!("cargo:rerun-if-env-changed=SOKSAK_GHOSTTY_VT_LIB");
}
