//! Test host: `cd gba && rustc --edition 2021 --test host/test_sysapps.rs -o /tmp/sysapps_t && /tmp/sysapps_t`
#![allow(dead_code)]
#[path = "../src/apps/software.rs"]
mod software;
#[path = "../src/apps/aiuto.rs"]
mod aiuto;
#[path = "../src/apps/tour.rs"]
mod tour;
#[path = "../src/apps/scanner.rs"]
mod scanner;
