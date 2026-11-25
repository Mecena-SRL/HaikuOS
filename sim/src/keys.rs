//! Chiavi per replica, derivate da una chiave madre dell'utente (simulazione host).
//! Schema: chiave_replica = primi 16 byte di Ascon-AEAD(chiave_madre, nonce = replica || epoca, msg = 16 zeri).
//! L'epoca cambia a ogni rotazione manuale. Da verificare: sostituire con una KDF dedicata prima dell'uso reale.
use ascon_aead::aead::{Aead, KeyInit, Payload};
use ascon_aead::AsconAead128;

pub fn replica_key(master: &[u8; 16], replica: u8, epoch: u32) -> [u8; 16] {
    let mut nonce = [0u8; 16];
    nonce[0] = replica;
    nonce[1..5].copy_from_slice(&epoch.to_le_bytes());
    let ct = AsconAead128::new(&(*master).into())
        .encrypt(&nonce.into(), Payload { msg: &[0u8; 16], aad: b"HKS-KDF" })
        .expect("cifratura KDF");
    let mut k = [0u8; 16];
    k.copy_from_slice(&ct[..16]);
    k
}
