//! Kart racing. Logica `no_std` (fisica, giri, oggetti, IA, gara, coppa) separata dal render.

#![cfg_attr(not(test), no_std)]

pub mod ai;
pub mod fx;
pub mod kart;
pub mod race;
pub mod render;
pub mod track;
