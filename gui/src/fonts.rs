//! Font di sistema incorporati nel binario. Default per ruolo: vedi `fonts/README.md`.
use crate::ttf::Font;

static UI: &[u8] = include_bytes!("../../fonts/Inter/Inter-Variable.ttf");

/// Font della UI (sans, default di sistema).
pub fn ui() -> Option<Font> {
    Font::parse(UI)
}
