//! Strumenti host di HaikuOS (std, non fanno parte del sistema).
//! - `manifest`: legge `haiku.toml` di un pacchetto.
//! - `build`: piano e esecuzione dei passi di compilazione.
//! - `pm`: indice, risoluzione dipendenze, installazione atomica.
//! - `ide`: buffer di testo con annulla (nucleo dell'IDE, senza GUI).
//! - `graph`: ordine di build con rilevamento dei cicli.
//! - `pkg`: verifica SHA-256 del payload di un pacchetto.

pub mod build;
pub mod graph;
pub mod ide;
pub mod pm;
pub mod manifest;
pub mod pkg;
