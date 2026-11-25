//! Chiavi per replica derivate dalla chiave madre, invio tra repliche e rotazione manuale.

use haiku_alloc_sim::keys::replica_key;
use haiku_alloc_sim::snapshot::{SnapError, Store};

const MADRE: [u8; 16] = *b"chiave-madre-ok!";

#[test]
fn stessi_ingressi_stessa_chiave() {
    assert_eq!(replica_key(&MADRE, 1, 0), replica_key(&MADRE, 1, 0));
}

#[test]
fn replica_epoca_e_madre_cambiano_la_chiave() {
    let k = replica_key(&MADRE, 1, 0);
    assert_ne!(k, replica_key(&MADRE, 2, 0));
    assert_ne!(k, replica_key(&MADRE, 1, 1));
    assert_ne!(k, replica_key(&[0; 16], 1, 0));
}

#[test]
fn replica_con_chiave_derivata_riceve_invio_altra_no() {
    let k1 = replica_key(&MADRE, 1, 0);
    let mut a = Store::new(k1);
    a.write("x", b"dati").unwrap();
    a.snapshot("s").unwrap();
    let d = a.diff(None, "s").unwrap();
    let mut b = Store::new(k1);
    b.apply(&d).unwrap();
    assert_eq!(b.read("x").unwrap(), b"dati");
    let mut c = Store::new(replica_key(&MADRE, 2, 0));
    assert_eq!(c.apply(&d), Err(SnapError::Corrupt));
}

#[test]
fn rotazione_mantiene_i_dati_e_cambia_la_chiave_di_invio() {
    let k0 = replica_key(&MADRE, 1, 0);
    let k1 = replica_key(&MADRE, 1, 1);
    let mut a = Store::new(k0);
    a.write("x", b"dati").unwrap();
    a.snapshot("s").unwrap();
    a.write("y", b"altro").unwrap();
    a.rotate_key(k1).unwrap();
    assert_eq!(a.read("x").unwrap(), b"dati");
    assert_eq!(a.read("y").unwrap(), b"altro");
    assert!(a.verify().is_ok());
    let d = a.diff(None, "s").unwrap();
    let mut vecchia = Store::new(k0);
    assert_eq!(vecchia.apply(&d), Err(SnapError::Corrupt));
    let mut nuova = Store::new(k1);
    nuova.apply(&d).unwrap();
    assert_eq!(nuova.read("x").unwrap(), b"dati");
}
