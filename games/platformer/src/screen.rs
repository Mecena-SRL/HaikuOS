//! Host grafico (solo std): titolo e pausa come overlay sul framebuffer ARGB. Il core non conosce menu.

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

fn rect(fb: &mut [u32], x: i32, y: i32, w: i32, h: i32, c: u32) {
    for yy in y.max(0)..(y + h).min(VIEW_H) {
        for xx in x.max(0)..(x + w).min(VIEW_W) {
            fb[(yy * VIEW_W + xx) as usize] = c;
        }
    }
}

/// Sovrappone il menu: fondo scurito, pannello, barra del titolo e pulsanti (senza font di testo).
pub fn overlay(fb: &mut [u32], s: Screen) {
    if s == Screen::Play {
        return;
    }
    for p in fb.iter_mut() {
        *p = 0xFF00_0000 | ((*p >> 2) & 0x003F_3F3F);
    }
    let (w, h) = (VIEW_W, VIEW_H);
    let (px, py, pw, ph) = (w / 8, h / 4, w * 3 / 4, h / 3);
    rect(fb, px, py, pw, ph, 0xFF18_1C28);
    rect(fb, px, py, pw, 6, 0xFFF0_C83C);
    let by = py + ph * 2 / 3;
    if s == Screen::Title {
        rect(fb, px + pw / 4, by, pw / 2, ph / 8, 0xFF50_C86E);
    } else {
        rect(fb, px + pw / 8, by, pw * 3 / 8, ph / 8, 0xFF50_C86E);
        rect(fb, px + pw * 9 / 16, by, pw * 5 / 16, ph / 8, 0xFFDC_4646);
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
