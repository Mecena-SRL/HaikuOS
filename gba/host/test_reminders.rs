//! Test host: `cd gba && rustc --edition 2021 --test host/test_reminders.rs -o /tmp/rem_t && /tmp/rem_t`
#![allow(dead_code)]
#[path = "../src/reminders.rs"]
mod reminders;
