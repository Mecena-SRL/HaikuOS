//! Test host: `cd gba && rustc --edition 2021 --test host/test_root.rs -o /tmp/gba_t && /tmp/gba_t`
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/persist.rs"]
mod persist;
#[path = "../src/alloc.rs"]
mod alloc;
#[path = "../src/monolith.rs"]
mod monolith;
#[path = "../src/workspace.rs"]
mod workspace;
#[path = "../src/space.rs"]
mod space;
#[path = "../src/desktop.rs"]
mod desktop;
#[path = "../src/font.rs"]
mod font;
