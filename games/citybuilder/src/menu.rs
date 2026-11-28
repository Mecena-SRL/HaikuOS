//! Schermate di avvio e pausa: sola navigazione, nessuna regola di gioco.

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

/// Titolo: un invio qualunque avvia la partita. Pausa: `r` o invio riprende, `q` esce.
pub fn on_menu_key(s: Screen, key: &str) -> Next {
    match (s, key) {
        (Screen::Title | Screen::Pause, "q") => Next::Quit,
        (Screen::Title, _) => Next::Go(Screen::Play),
        (Screen::Pause, "r" | "") => Next::Go(Screen::Play),
        _ => Next::Stay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titolo_avvia_pausa_riprende_ed_esce() {
        assert_eq!(on_menu_key(Screen::Title, ""), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Title, "q"), Next::Quit);
        assert_eq!(on_menu_key(Screen::Pause, "r"), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Pause, "t"), Next::Stay);
        assert_eq!(on_menu_key(Screen::Pause, "q"), Next::Quit);
    }
}
