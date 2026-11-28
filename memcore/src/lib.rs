//! Primitive di memoria del kernel (docs/kernel-study.md, righe 12-14).
//! Logica pura, no_std, senza heap: strutture a dimensione fissa, decisa a compilazione.
#![cfg_attr(not(test), no_std)]

pub mod buddy;
pub mod pool;
pub mod prot;

pub use buddy::Buddy;
pub use pool::Pool;
pub use prot::Prot;
