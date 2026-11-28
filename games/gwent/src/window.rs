//! Finestra reale (feature `window`, minifb). Stesso tabellone di `view`.
//! Mouse: il passaggio sopra una carta la seleziona solo quando il mouse si muove; un click gioca o passa.
//! Tastiera: frecce spostano, invio gioca (o inizia una nuova partita a fine partita), `p` passa, Esc esce.

use crate::view::{self, Hit, Pane};
use crate::Ui;
use gwent::Act;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

pub const WIN_W: usize = 1280;
pub const WIN_H: usize = 720;

pub fn run(mut ui: Ui) {
    // Dimensione iniziale: 1280x720, oppure `GWENT_WIN=800x480` per provare altri schermi.
    let (ww, wh) = std::env::var("GWENT_WIN")
        .ok()
        .and_then(|v| {
            let (a, b) = v.split_once('x')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .unwrap_or((WIN_W, WIN_H));
    let opts = WindowOptions { resize: true, ..WindowOptions::default() };
    let mut win = Window::new("Gwent", ww, wh, opts).expect("finestra");
    win.set_target_fps(60);
    let mut pane = Pane::new(ww, wh);
    let mut was_down = false;
    let mut last_mouse: Option<(f32, f32)> = None;

    while win.is_open() {
        let (w, h) = win.get_size();
        if w != pane.im.w || h != pane.im.h {
            pane = Pane::new(w.max(1), h.max(1));
        }
        let n = ui.g.side[0].hand.n as usize;
        for k in win.get_keys_pressed(KeyRepeat::No) {
            if k == Key::Escape {
                return;
            }
            if ui.g.over.is_some() {
                if k == Key::Enter {
                    ui.restart();
                }
                continue;
            }
            match k {
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

        let mouse = win.get_mouse_pos(MouseMode::Clamp);
        let moved = mouse != last_mouse;
        last_mouse = mouse;
        let down = win.get_mouse_down(MouseButton::Left);
        let click = down && !was_down;
        was_down = down;

        if let Some((mx, my)) = mouse {
            let (dx, dy) = pane.to_design(mx as i32, my as i32);
            if moved && ui.g.over.is_none() {
                if let Some(Hit::Card(i)) = view::hit(dx, dy, n, ui.cur) {
                    ui.cur = i;
                }
            }
            if click {
                if ui.g.over.is_some() {
                    ui.restart();
                } else {
                    match view::hit(dx, dy, n, ui.cur) {
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
        }
        view::frame_into(&mut pane, &ui.g, ui.cur, &ui.msg);
        win.update_with_buffer(&pane.im.px, pane.im.w, pane.im.h).expect("disegno");
    }
}
