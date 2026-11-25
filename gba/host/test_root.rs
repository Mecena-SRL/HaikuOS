//! Test host: `cd gba && rustc --edition 2021 --test host/test_root.rs -o /tmp/gba_t && /tmp/gba_t`
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/persist.rs"]
mod persist;
