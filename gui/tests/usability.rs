// Test di usabilità: ogni azione comune in pochi tasti.
mod common;
use haiku_gui::{Event, Icon, Key, Shell};

static ICONS: [Icon; 9] = [
    Icon { label: "FILE", app: 1, title: "FILE", w: 120, h: 80 },
    Icon { label: "TERM", app: 2, title: "TERMINALE", w: 120, h: 80 },
    Icon { label: "CALC", app: 3, title: "CALCOLATRICE", w: 80, h: 80 },
    Icon { label: "NOTE", app: 4, title: "NOTE", w: 120, h: 80 },
    Icon { label: "CLOCK", app: 5, title: "ORA", w: 80, h: 40 },
    Icon { label: "SET", app: 6, title: "IMPOST", w: 120, h: 80 },
    Icon { label: "READ", app: 7, title: "LETTORE", w: 120, h: 80 },
    Icon { label: "CHESS", app: 8, title: "SCACCHI", w: 120, h: 80 },
    Icon { label: "GAME", app: 9, title: "GIOCHI", w: 120, h: 80 },
];

fn press(s: &mut Shell, keys: &[Key]) -> Event {
    let mut e = Event::None;
    for &k in keys {
        e = s.key(k);
    }
    e
}

#[test]
fn any_app_opens_in_at_most_five_keys() {
    for target in 0..ICONS.len() {
        let mut s = Shell::new(&ICONS, 240, 160);
        let (mut keys, mut cur) = (Vec::new(), 0usize);
        while cur != target {
            if cur < target && (cur % 4) < (target % 4) { cur += 1; keys.push(Key::Right) }
            else if cur > target && (cur % 4) > (target % 4) { cur -= 1; keys.push(Key::Left) }
            else if cur < target { cur += 4; keys.push(Key::Down) }
            else { cur -= 4; keys.push(Key::Up) }
        }
        keys.push(Key::A);
        assert!(keys.len() <= 5, "icona {} richiede {} tasti", target, keys.len());
        assert_eq!(press(&mut s, &keys), Event::Launch(ICONS[target].app));
        assert_eq!(s.wm.top().unwrap().title, ICONS[target].title);
    }
}

#[test]
fn close_window_is_one_key_b() {
    let mut s = Shell::new(&ICONS, 240, 160);
    press(&mut s, &[Key::A]);
    assert_eq!(s.key(Key::B), Event::Closed(1));
    assert!(s.wm.top().is_none());
}

#[test]
fn switch_window_is_one_key_select() {
    let mut s = Shell::new(&ICONS, 240, 160);
    press(&mut s, &[Key::A, Key::Start, Key::Right, Key::A]);
    assert_eq!(s.wm.top().unwrap().app, 2);
    s.key(Key::Select);
    assert_eq!(s.wm.top().unwrap().app, 1);
}

#[test]
fn open_window_gets_arrows_and_reopen_raises_instead_of_duplicating() {
    let mut s = Shell::new(&ICONS, 240, 160);
    press(&mut s, &[Key::A, Key::Start, Key::Right, Key::A]);
    assert_eq!(s.key(Key::Down), Event::ToApp(Key::Down));
    press(&mut s, &[Key::Start, Key::Left, Key::A]); // app 1 già aperta: va in primo piano
    assert_eq!(s.wm.len(), 2);
    assert_eq!(s.wm.top().unwrap().app, 1);
}

#[test]
fn pointer_clicks_work_on_icons_close_and_content() {
    let mut s = Shell::new(&ICONS, 240, 160);
    assert_eq!(s.click(8 + 48, 24), Event::Launch(2));
    let f = s.wm.top().unwrap().frame;
    let c = haiku_gui::layout::content(f);
    assert_eq!(s.click(c.x + 1, c.y + 2), Event::Tap { app: 2, x: 1, y: 2 });
    let cb = haiku_gui::layout::close_button(f);
    assert_eq!(s.click(cb.x + 1, cb.y + 1), Event::Closed(2));
}
