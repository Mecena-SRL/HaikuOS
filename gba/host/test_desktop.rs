//! Test host: `cd gba && rustc --edition 2021 --test host/test_desktop.rs -o /tmp/desk_t && /tmp/desk_t`
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/font.rs"]
mod font;
#[path = "../src/terminal.rs"]
mod terminal;
#[path = "../src/calc.rs"]
mod calc;
#[path = "../src/calc_ui.rs"]
mod calc_ui;
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/chess.rs"]
mod chess;
#[path = "../src/media_ui.rs"]
mod media_ui;
#[path = "../src/apps/mod.rs"]
mod apps;
#[path = "../src/editor.rs"]
mod editor;
#[path = "../src/reminders.rs"]
mod reminders;
#[path = "../src/desktop.rs"]
mod desktop;
