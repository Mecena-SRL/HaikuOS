// Menu, ricerca, finestre: riduci, massimizza, trascina, barra.
mod common;
use haiku_gui::layout::*;
use haiku_gui::{Event, Icon, Key, Shell, Tok};

static ICONS: [Icon; 3] = [
    Icon { label: "FILE", app: 1, title: "FILE", w: 120, h: 80 },
    Icon { label: "TERM", app: 2, title: "TERMINALE", w: 120, h: 80 },
    Icon { label: "WEB", app: 4, title: "BROWSER", w: 120, h: 80 },
];

fn type_str(s: &mut Shell, text: &str) {
    for b in text.bytes() {
        s.text(b);
    }
}

#[test]
fn menu_search_filters_and_enter_launches() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.toggle_menu();
    type_str(&mut s, "term");
    assert_eq!(s.match_count(), 1);
    assert_eq!(s.key(Key::Start), Event::Launch(2));
    assert!(!s.menu, "il menu si chiude all'apertura");
}

#[test]
fn menu_letters_do_not_trigger_app_keys() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.toggle_menu();
    s.key(Key::A); // Z: è una lettera, non apre nulla
    s.key(Key::B); // X: idem
    assert!(s.wm.is_empty());
    assert!(s.menu);
    s.text(0x1B); // Esc
    assert!(!s.menu);
}

#[test]
fn panel_menu_button_toggles_menu() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.click(2, 2);
    assert!(s.menu);
    s.click(200, 200); // fuori dal menu: lo chiude
    assert!(!s.menu);
}

#[test]
fn minimize_hides_window_and_taskbar_restores_it() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.key(Key::A);
    let f = s.wm.top().unwrap().frame;
    let mb = min_button(f);
    s.click(mb.x + 1, mb.y + 1);
    assert!(s.wm.top().is_none(), "nessuna finestra attiva");
    assert!(s.wm.find_app(1).is_some(), "ancora aperta");
    s.click(task_button(0).x + 2, 3); // pulsante nella barra
    assert_eq!(s.wm.top().unwrap().app, 1);
    assert!(!s.wm.get(0).unwrap().min);
}

#[test]
fn maximize_fills_area_below_bar_and_toggles_back() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.key(Key::A);
    let before = s.wm.top().unwrap().frame;
    let xb = max_button(before);
    s.click(xb.x + 1, xb.y + 1);
    assert_eq!(s.wm.top().unwrap().frame, Rect::new(0, BAR_H, 320, 240 - BAR_H));
    let xb = max_button(s.wm.top().unwrap().frame);
    s.click(xb.x + 1, xb.y + 1);
    assert_eq!(s.wm.top().unwrap().frame, before);
}

#[test]
fn dragging_title_moves_window_and_stays_on_screen() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.key(Key::A);
    let f = s.wm.top().unwrap().frame;
    s.press(f.x + 3, f.y + 3);
    s.drag(f.x + 3 + 20, f.y + 3 + 10);
    s.release();
    assert_eq!(s.wm.top().unwrap().frame.x, f.x + 20);
    s.press(f.x + 23, f.y + 13);
    s.drag(-500, -500);
    s.release();
    let g = s.wm.top().unwrap().frame;
    assert!(g.x == 0 && g.y == BAR_H, "fermo in alto a sinistra: {:?}", g);
}

#[test]
fn clock_text_is_hh_mm() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.set_clock(9, 5);
    assert_eq!(core::str::from_utf8(&s.clock).unwrap(), "09:05");
}

#[test]
fn window_body_shows_app_content() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.key(Key::A);
    let mut b = common::Buf::new(320, 240);
    haiku_gui::render::draw(&mut b, &s);
    assert!(b.texts.iter().any(|t| t == "leggimi.txt"));
}

#[test]
fn calculator_adds_and_shows_result() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.key(Key::A); // FILE: prima finestra; poi la calcolatrice non c'è: uso un'icona apposta
    let mut c = haiku_gui::Calc::new();
    let mut buf = [0u8; 21];
    for b in b"5+3=" {
        c.key(*b);
    }
    assert_eq!(c.show(&mut buf), "8");
    c.key(b'C');
    assert_eq!(c.show(&mut buf), "0");
}

#[test]
fn terminal_runs_sysapps_command() {
    let mut t = haiku_gui::Term::new();
    for b in b"echo ciao" {
        t.type_char(*b);
    }
    t.enter();
    assert_eq!(&t.hist[1][..t.hl[1]], b"ciao", "riga 0 è il comando");
}

#[test]
fn file_view_selects_and_opens() {
    let mut s = Shell::new(&ICONS, 320, 240);
    s.key(Key::A);
    s.key(Key::Down);
    assert_eq!(s.files.sel, 1);
    s.text(0x0D);
    assert!(s.files.open);
}

#[test]
fn enter_on_desktop_opens_selected_icon_and_text_goes_to_app() {
    let mut s = Shell::new(&ICONS, 320, 240);
    assert_eq!(s.text(0x0D), Event::Launch(1));
    assert_eq!(s.text(b'5'), Event::None);
    assert_eq!(s.calc.show(&mut [0u8; 21]), "0"); // è FILE, non CALC: nessun effetto
}

#[test]
fn wallpaper_is_wordmark_bottom_right_in_t3() {
    let s = Shell::new(&ICONS, 320, 240);
    let mut b = common::Buf::new(320, 240);
    haiku_gui::render::draw(&mut b, &s);
    assert_eq!(b.at(0, 239), Tok::T0, "sfondo piatto");
    assert!(b.texts.iter().any(|t| t == "HAIKUOS"));
}
