//! Sistema grafico comune (bibbia grafica: docs/graphic-bible.md).
//! Logica pura, no_std, senza allocazione. Il disegno passa dal trait `Canvas`:
//! GBA e x86 implementano solo `fill` e `text`; regole, layout e input sono qui.
#![no_std]

pub mod anim;
pub mod apps;
#[path = "../../gba/src/chess.rs"]
pub mod chess;
pub mod icons;
mod icon_vec_data;
pub mod input;
pub mod layout;
pub mod palette;
pub mod render;
pub mod vfont;
mod vfont_data;
pub mod shell;
pub mod wm;

pub use apps::{Calc, FileView, Term};
pub use input::Key;
pub use palette::Tok;
pub use shell::{Event, Icon, Shell};
