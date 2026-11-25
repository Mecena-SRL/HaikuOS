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

fn type_str(t: &mut Terminal, s: &[u8]) {
    for &c in s {
        t.type_byte(c);
    }
}

#[test]
fn typed_ls_lists_root() {
    let f = sample();
    let mut t = Terminal::new();
    type_str(&mut t, b"LS");
    t.run_input(&f);
    assert_eq!(t.lines(), 2);
    assert_eq!(text(&t, 0), b"docs");
    assert_eq!(t.input(), b"");
}

#[test]
fn typed_cd_ignores_case_and_up_returns() {
    let f = sample();
    let mut t = Terminal::new();
    type_str(&mut t, b"CD DOCS");
    t.run_input(&f);
    assert_eq!(t.cwd(), b"docs");
    type_str(&mut t, b"up");
    t.run_input(&f);
    assert_eq!(t.cwd(), b"");
}

#[test]
fn typed_errors_are_shown() {
    let f = sample();
    let mut t = Terminal::new();
    type_str(&mut t, b"XYZ");
    t.run_input(&f);
    assert_eq!(text(&t, 0), b"ERRORE");
    type_str(&mut t, b"CD NOPE");
    t.run_input(&f);
    assert_eq!(text(&t, 0), b"NON TROVATO");
    assert_eq!(t.cwd(), b"");
}

#[test]
fn input_editing_pick_and_backspace() {
    let mut t = Terminal::new();
    t.toggle_typing();
    t.pick_step(-1); // dal primo carattere (spazio) all'ultimo di ALPHA
    t.type_pick();
    assert_eq!(t.input(), &terminal::ALPHA[terminal::ALPHA.len() - 1..]);
    t.type_byte(b'A');
    t.backspace();
    t.backspace();
    assert_eq!(t.input(), b"");
    for _ in 0..(terminal::INPUT_MAX + 4) {
        t.type_byte(b'A');
    }
    assert_eq!(t.input().len(), terminal::INPUT_MAX);
}

#[test]
fn comandi_chess_e_read_richiedono_la_finestra() {
    let f = sample();
    let mut t = Terminal::new();
    assert_eq!(t.take_open(), None);
    t.toggle_typing();
    for c in b"CHESS" {
        let i = terminal::ALPHA.iter().position(|a| a == c).unwrap();
        while t.pick != i { t.pick_step(1); }
        t.type_pick();
    }
    t.run_input(&f);
    assert_eq!(t.take_open(), Some(terminal::Open::Chess));
    assert_eq!(t.take_open(), None);
    assert_eq!(t.line(0), b"SCACCHI");
}

#[test]
fn typed_app_names_request_their_icon() {
    let f = sample();
    let mut t = Terminal::new();
    for (name, icon) in [(&b"calc"[..], 2), (b"FILES", 0), (b"Remind", 8), (b"settings", 5)] {
        type_str(&mut t, name);
        t.run_input(&f);
        assert_eq!(t.take_launch(), Some(icon));
        assert_eq!(t.take_launch(), None);
    }
}

#[test]
fn unknown_cd_target_is_not_an_app() {
    let f = sample();
    let mut t = Terminal::new();
    type_str(&mut t, b"CD NOPE");
    t.run_input(&f);
    assert_eq!(t.take_launch(), None);
    assert_eq!(text(&t, 0), b"NON TROVATO" as &[u8]);
}

#[test]
fn typed_add_takes_the_reminder_text() {
    let f = sample();
    let mut t = Terminal::new();
    type_str(&mut t, b"ADD DAD");
    t.run_input(&f);
    assert_eq!(t.line(0), b"PROMEMORIA OK");
    let (buf, n) = t.take_add().unwrap();
    assert_eq!(&buf[..n], b"DAD");
    assert_eq!(t.take_add(), None);
}
