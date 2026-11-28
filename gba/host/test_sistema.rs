//! Test host: `cd gba && rustc --edition 2021 --test host/test_sistema.rs -o /tmp/sis_t && /tmp/sis_t`
#![allow(dead_code)]
#[path = "../src/apps/sysutil.rs"]
mod sysutil;
#[path = "../src/apps/registri.rs"]
mod registri;
#[path = "../src/apps/connessioni.rs"]
mod connessioni;
#[path = "../src/apps/dischi.rs"]
mod dischi;
#[path = "../src/apps/analisi.rs"]
mod analisi;
#[path = "../src/apps/caratteri.rs"]
mod caratteri;
