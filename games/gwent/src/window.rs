//! Finestra reale (feature `window`, minifb). Stesso tabellone di `view`: mouse (passaggio = selezione,
//! click = gioca o passa) e tastiera (frecce sposta, invio gioca, `p` passa, Esc esce).

use crate::view::{self, Hit, Pane};
use crate::Ui;
use gwent::Act;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

pub const WIN_W: usize = 1280;
pub const WIN_H: usize = 720;

pub fn run(mut ui: Ui) {
    let opts = WindowOptions { resize: true, ..WindowOptions::default() };
    let mut win = Window::new("Gwent", WIN_W, WIN_H, opts).expect("finestra");
    win.set_target_fps(60);
    let mut pane = Pane::new(WIN_W, WIN_H);
    let mut was_down = false;

    while win.is_open() {
        let (w, h) = win.get_size();
        if w != pane.im.w || h != pane.im.h {
            pane = Pane::new(w.max(1), h.max(1));
        }
        let n = ui.g.side[0].hand.n as usize;
        for k in win.get_keys_pressed(KeyRepeat::No) {
            match k {
                Key::Escape => return,
                Key::Left => ui.cur = ui.cur.saturating_sub(1),
                Key::Right => ui.cur = (ui.cur + 1).min(n.saturating_sub(1)),
                Key::Enter => {
                    ui.play_cur();
                }
                Key::P => {
                    ui.human(Act::Pass);
                }
                _ => {}
            }
        }
        let down = win.get_mouse_down(MouseButton::Left);
        let click = down && !was_down;
        was_down = down;
        if let Some((mx, my)) = win.get_mouse_pos(MouseMode::Clamp) {
            let (dx, dy) = pane.to_design(mx as i32, my as i32);
            // Passaggio del mouse sopra una carta: la seleziona.
            if let Some(Hit::Card(i)) = view::hit(dx, dy, n) {
                ui.cur = i;
            }
            if click {
                match view::hit(dx, dy, n) {
                    Some(Hit::Pass) => {
                        ui.human(Act::Pass);
                    }
                    Some(Hit::Card(i)) => {
                        ui.cur = i;
                        ui.human(Act::Play(i));
                    }
                    None => {}
                }
            }
        }
        view::frame_into(&mut pane, &ui.g, ui.cur, &ui.msg);
        win.update_with_buffer(&pane.im.px, pane.im.w, pane.im.h).expect("disegno");
    }
}
