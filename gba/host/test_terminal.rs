//! Test host: `cd gba && rustc --edition 2021 --test host/test_terminal.rs -o /tmp/term_t && /tmp/term_t`
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/terminal.rs"]
mod terminal;

use fs::Fs;
use terminal::Terminal;

/// Radice: docs/ (con a, b) e file "c". Menu: LS, CD .., CD docs.
fn sample() -> Fs {
    let mut f = Fs::new();
    f.mkdir(b"docs", 0).unwrap();
    f.create(b"docs/a", 0).unwrap();
    f.create(b"docs/b", 0).unwrap();
    f.create(b"c", 0).unwrap();
    f
}

fn text(t: &Terminal, k: usize) -> &[u8] {
    t.line(k)
}

#[test]
fn menu_has_ls_up_and_only_dirs() {
    let f = sample();
    let t = Terminal::new();
    assert_eq!(t.menu_len(&f), 3);
    let mut lab = [0u8; terminal::LABEL_MAX];
    let n = t.label(&f, 2, &mut lab);
    assert_eq!(&lab[..n], b"CD docs");
    let n = t.label(&f, 0, &mut lab);
    assert_eq!(&lab[..n], b"LS");
}

#[test]
fn ls_lists_children_of_root() {
    let f = sample();
    let mut t = Terminal::new();
    t.run(&f, 0);
    assert_eq!(t.lines(), 2);
    assert_eq!(text(&t, 0), b"docs");
    assert_eq!(text(&t, 1), b"c");
}

#[test]
fn cd_then_ls_then_up() {
    let f = sample();
    let mut t = Terminal::new();
    t.run(&f, 2); // cd docs
    assert_eq!(t.cwd(), b"docs");
    assert_eq!(t.menu_len(&f), 2); // LS, CD .. (nessuna sottodirectory)
    t.run(&f, 0);
    assert_eq!(t.lines(), 2);
    assert_eq!(text(&t, 0), b"a");
    assert_eq!(text(&t, 1), b"b");
    t.run(&f, 1); // cd ..
    assert_eq!(t.cwd(), b"");
    assert_eq!(t.lines(), 0);
}

#[test]
fn ls_empty_dir_says_vuoto() {
    let mut f = Fs::new();
    f.mkdir(b"vuota", 0).unwrap();
    let mut t = Terminal::new();
    t.run(&f, 2); // cd vuota
    t.run(&f, 0);
    assert_eq!(text(&t, 0), b"VUOTO");
}

#[test]
fn selection_wraps_around() {
    let f = sample();
    let mut t = Terminal::new();
    t.step_sel(&f, -1);
    assert_eq!(t.sel, 2);
    t.step_sel(&f, 1);
    assert_eq!(t.sel, 0);
}

#[test]
fn out_of_range_command_does_nothing() {
    let f = sample();
    let mut t = Terminal::new();
    t.run(&f, 9);
    assert_eq!(t.cwd(), b"");
    assert_eq!(t.lines(), 0);
}
