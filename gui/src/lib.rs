//! Sistema grafico comune (bibbia grafica: docs/graphic-bible.md).
//! Logica pura, no_std, senza allocazione. Il disegno passa dal trait `Canvas`:
//! GBA e x86 implementano solo `fill` e `text`; regole, layout e input sono qui.
#![no_std]

pub mod apps;
pub mod icons;
pub mod input;
pub mod layout;
pub mod palette;
pub mod render;
pub mod shell;
pub mod wm;

pub use apps::{Calc, FileView, Term};
pub use input::Key;
pub use palette::Tok;
pub use shell::{Event, Icon, Shell};
