//! Suite office di HaikuOS: moduli senza disegno, testabili sull'host.
#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

pub mod inflate;
pub mod text;
pub mod zip;
pub mod formats;
pub mod pdf;
