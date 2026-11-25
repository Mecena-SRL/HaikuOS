//! Test host: `cd gba && rustc --edition 2021 --test host/test_workspace.rs -o /tmp/ws_t && /tmp/ws_t`
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/workspace.rs"]
mod workspace;
