use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use unicode_width::UnicodeWidthStr;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/stress")
        .join(name)
}

/// Runs `mar --cat` with only the built-in defaults, whatever the user's config says.
fn mar(file: &str, color: &str, width: usize) -> Output {
    let width = width.to_string();
    Command::new(env!("CARGO_BIN_EXE_mar"))
        .args(["--cat", "--color", color, "--width", &width])
        .arg(fixture(file))
        .env("XDG_CONFIG_HOME", fixture("no-config"))
        .env_remove("NO_COLOR")
        .output()
        .expect("mar runs")
}

fn fixtures() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixture(""))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| {
            n.ends_with(".md") && n != "README.md" && n != "big.md" && n != "invalid-utf8.md"
        })
        .collect();
    names.sort();
    assert!(names.len() >= 10, "fixtures missing: {names:?}");
    names
}

#[test]
fn every_fixture_renders_cleanly() {
    for name in fixtures() {
        for color in ["always", "never"] {
            for width in [20, 60, 100] {
                let out = mar(&name, color, width);
                let err = String::from_utf8_lossy(&out.stderr);
                assert!(
                    out.status.success(),
                    "{name} --color {color} --width {width}: {err}"
                );
                assert!(
                    err.is_empty(),
                    "{name} --color {color} --width {width} wrote to stderr: {err}"
                );
            }
        }
    }
}

#[test]
fn plain_output_has_no_control_characters() {
    for name in fixtures() {
        let out = mar(&name, "never", 60).stdout;
        let text = String::from_utf8(out).expect("UTF-8 output");
        let bad = text.chars().find(|&c| c.is_control() && c != '\n');
        assert_eq!(bad, None, "{name} leaked a control character");
    }
}

#[test]
fn injected_escapes_are_neutralized() {
    let out = mar("nasty.md", "always", 80).stdout;
    for attack in [
        &b"\x1b]52;c;cGF3"[..],
        b"\x1b]0;owned",
        b"\x1b[2J",
        b"\x1b[10;10H",
        b"\x1b[31mRED",
        b"\xc2\x9b",
    ] {
        assert!(
            !out.windows(attack.len()).any(|w| w == attack),
            "{:?} got through",
            String::from_utf8_lossy(attack)
        );
    }
}

#[test]
fn wide_blocks_are_clipped_in_cat_output() {
    let text = String::from_utf8(mar("wide.md", "never", 60).stdout).unwrap();
    assert!(!text.contains("END-SHOULD-NOT-BE-VISIBLE"));
    let long = text
        .lines()
        .filter(|l| l.contains("much longer"))
        .collect::<Vec<_>>();
    assert!(!long.is_empty());
    assert!(
        long.iter().all(|l| l.width() <= 60 && l.contains('…')),
        "{long:#?}"
    );
}

#[test]
fn broken_diagrams_show_their_source() {
    let text = String::from_utf8(mar("mermaid.md", "never", 100).stdout).unwrap();
    assert!(text.contains("class Café"));
    assert!(text.contains("notADiagramType"));
}

#[test]
fn invalid_utf8_is_a_clean_error() {
    let out = mar("invalid-utf8.md", "never", 60);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("UTF-8"));
}

#[test]
fn empty_input_prints_nothing() {
    assert!(mar("empty.md", "always", 60).stdout.is_empty());
}
