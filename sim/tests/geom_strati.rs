use haiku_alloc_sim::geom::*;
use haiku_alloc_sim::storage::{Device, StorageError};

#[test]
fn offset_mappa_la_partizione() {
    let mut d = Device::new(8, 4);
    d.write(3, &[7, 7, 7, 7]).unwrap();
    let p = Offset::new(d, 2, 4).unwrap();
    let mut buf = [0u8; 4];
    p.read(1, &mut buf).unwrap();
    assert_eq!(buf, [7, 7, 7, 7]);
    assert_eq!(p.blocks(), 4);
    assert_eq!(p.read(4, &mut buf), Err(StorageError::OutOfRange));
}

#[test]
fn offset_rifiuta_finestra_fuori_disco() {
    let d = Device::new(4, 4);
    assert!(Offset::new(d, 3, 2).is_none());
}

#[test]
fn sola_lettura_blocca_scritture_senza_toccare_il_disco() {
    let mut r = ReadOnly(Device::new(2, 4));
    assert_eq!(r.write(0, &[1, 2, 3, 4]), Err(StorageError::Halted));
    let mut buf = [9u8; 4];
    r.read(0, &mut buf).unwrap();
    assert_eq!(buf, [0, 0, 0, 0]);
}

#[test]
fn partizione_su_sola_lettura() {
    let mut p = Offset::new(ReadOnly(Device::new(6, 4)), 1, 3).unwrap();
    assert_eq!(p.write(0, &[1; 4]), Err(StorageError::Halted));
    let mut buf = [0u8; 4];
    assert!(p.read(2, &mut buf).is_ok());
}

#[test]
fn lettura_con_buffer_di_dimensione_errata() {
    let d = Device::new(1, 4);
    let mut buf = [0u8; 3];
    assert_eq!(Block::read(&d, 0, &mut buf), Err(StorageError::BadSize));
}
