//! Host grafico (solo std): titolo e pausa come overlay ARGB disegnati con `haiku-gui::Canvas` (canvas `render::Fb`).

use haiku_gui::layout::{Rect, MENU_ROW};
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use platformer::render::Fb;
use platformer::{VIEW_H, VIEW_W};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Play,
    Pause,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    Go(Screen),
    Quit,
    Stay,
}

/// Titolo: invio avvia. Pausa: `r` o invio riprende, `q` esce.
pub fn on_menu_key(s: Screen, key: &str) -> Next {
    match (s, key) {
        (Screen::Title | Screen::Pause, "q") => Next::Quit,
        (Screen::Title, _) => Next::Go(Screen::Play),
        (Screen::Pause, "r" | "") => Next::Go(Screen::Play),
        _ => Next::Stay,
    }
}

/// Sovrappone il menu: fondo scurito, pannello con titolo e voci (la prima selezionata).
/// Le voci sono centrate nella riga: `Canvas::menu` di gui mette il testo fuori riga.
pub fn overlay(fb: &mut [u32], s: Screen) {
    if s == Screen::Play {
        return;
    }
    for p in fb.iter_mut() {
        *p = 0xFF00_0000 | ((*p >> 2) & 0x003F_3F3F);
    }
    let (title, rows): (&str, &[&str]) = match s {
        Screen::Title => ("PLATFORMER", &["INIZIA", "ESCI"]),
        _ => ("PAUSA", &["RIPRENDI", "ESCI"]),
    };
    let mut c = Fb(fb);
    let p = Rect::new(VIEW_W / 8, VIEW_H / 4, VIEW_W * 3 / 4, 22 + MENU_ROW * rows.len() as i32 + 4);
    c.fill(p, Tok::T0);
    c.fill(Rect::new(p.x, p.y, p.w, 3), Tok::A(1));
    c.text(p.x + 6, p.y + 6, title, Tok::A(1));
    for (i, label) in rows.iter().enumerate() {
        let y = p.y + 18 + i as i32 * MENU_ROW;
        let (bg, fg) = if i == 0 { (Tok::T2, Tok::T0) } else { (Tok::T0, Tok::T2) };
        c.fill(Rect::new(p.x + 4, y, p.w - 8, MENU_ROW), bg);
        c.text(p.x + 8, y + 2, label, fg);
    }
}

/// Testo dei menu per il terminale.
pub fn menu_text(s: Screen) {
    match s {
        Screen::Title => println!("PLATFORMER\nINVIO per iniziare, q per uscire"),
        Screen::Pause => println!("PAUSA: r riprendi, q esci"),
        Screen::Play => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titolo_pausa_e_tasti() {
        assert_eq!(on_menu_key(Screen::Title, ""), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Title, "q"), Next::Quit);
        assert_eq!(on_menu_key(Screen::Pause, "r"), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Pause, "x"), Next::Stay);
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// Misura il costo di un fotogramma (mondo + HUD + menu): `cargo test --release -- --ignored --nocapture bench`.
    #[test]
    #[ignore]
    fn bench_frame() {
        let g = platformer::Game::from_src(platformer::levels::LEVELS[0]);
        let mut fb = vec![0u32; (VIEW_W * VIEW_H) as usize];
        let t = std::time::Instant::now();
        for _ in 0..2000 {
            platformer::render::draw(&g, &mut fb);
            overlay(&mut fb, Screen::Pause);
            std::hint::black_box(&fb);
        }
        println!("BENCH platformer {:.1} us/frame", t.elapsed().as_secs_f64() * 1e6 / 2000.0);
    }
}
