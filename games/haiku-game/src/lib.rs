//! Core dei giochi HaikuOS: matematica intera, RNG, passo fisso, esiti di schermata.
//! `no_std`, nessuna allocazione. Il PPM (solo host) sta dietro la feature `host`.

#![cfg_attr(not(feature = "host"), no_std)]

pub mod clock;
pub mod math;
pub mod rng;
pub mod scene;

#[cfg(feature = "host")]
pub mod ppm;
