//! Test host: `cd gba && rustc --edition 2021 --test host/test_vault.rs -o /tmp/vault_t && /tmp/vault_t`
#![allow(dead_code)]
#[path = "../src/ascon.rs"]
mod ascon;
#[path = "../src/apps/passwords.rs"]
mod passwords;
#[path = "../src/apps/vault.rs"]
mod vault;
