//! Primo avvio di HaikuOS: validazione della configurazione iniziale
//! e creazione dell'utente amministratore. La password non viene mai
//! conservata in chiaro: solo come hash PBKDF2-SHA256 con sale.
#![cfg_attr(not(test), no_std)]

pub mod config;
pub mod password;
pub mod text;
pub mod wizard;

#[cfg(test)]
mod tests;
