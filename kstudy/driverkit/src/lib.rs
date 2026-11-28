//! Driver scritto una volta sola contro il trait `Bus` (vedi docs/kernel-study.md, riga 19).
//! Il modulo `driver` non usa heap ne' tipi del kernel: compila no_std e gira sull'host.
#![cfg_attr(not(test), no_std)]

pub mod bus;
pub mod driver;
