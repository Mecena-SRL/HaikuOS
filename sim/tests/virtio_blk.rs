use haiku_alloc_sim::storage::Device;
use haiku_alloc_sim::virtio_blk::*;

fn settore(v: u8) -> [u8; SECTOR] {
    [v; SECTOR]
}

#[test]
fn scrittura_poi_lettura_in_ordine() {
    let mut disco = Device::new(4, SECTOR);
    let mut blk = VirtioBlk::new();
    let w = blk.submit(Op::Write, 2, settore(5)).unwrap();
    let r = blk.submit(Op::Read, 2, settore(0)).unwrap();
    assert_eq!(blk.process(&mut disco), 2);
    let scritto = blk.complete().unwrap();
    assert_eq!((scritto.id, scritto.status), (w, Status::Ok));
    let letto = blk.complete().unwrap();
    assert_eq!((letto.id, letto.status), (r, Status::Ok));
    assert_eq!(letto.data, settore(5));
}

#[test]
fn settore_fuori_disco_da_errore_senza_fermare_la_coda() {
    let mut disco = Device::new(1, SECTOR);
    let mut blk = VirtioBlk::new();
    blk.submit(Op::Read, 9, settore(0)).unwrap();
    let ok = blk.submit(Op::Write, 0, settore(1)).unwrap();
    blk.process(&mut disco);
    assert_eq!(blk.complete().unwrap().status, Status::IoErr);
    let d = blk.complete().unwrap();
    assert_eq!((d.id, d.status), (ok, Status::Ok));
}

#[test]
fn coda_piena_rifiuta_senza_perdere_richieste() {
    let mut blk = VirtioBlk::new();
    for _ in 0..QUEUE {
        blk.submit(Op::Read, 0, settore(0)).unwrap();
    }
    assert_eq!(blk.submit(Op::Read, 0, settore(0)), Err(BlkError::Full));
    let mut disco = Device::new(1, SECTOR);
    assert_eq!(blk.process(&mut disco), QUEUE);
}

#[test]
fn completamenti_pieni_tengono_le_richieste_in_coda() {
    let mut disco = Device::new(1, SECTOR);
    let mut blk = VirtioBlk::new();
    for _ in 0..QUEUE + 2 {
        let _ = blk.submit(Op::Read, 0, settore(0));
    }
    assert_eq!(blk.process(&mut disco), QUEUE);
    // Completamenti pieni: nessun progresso finché il programma non ritira.
    assert_eq!(blk.process(&mut disco), 0);
    while blk.complete().is_some() {}
    assert_eq!(blk.process(&mut disco), 0);
}

#[test]
fn id_mai_zero() {
    let mut blk = VirtioBlk::new();
    let mut disco = Device::new(1, SECTOR);
    for _ in 0..3 * QUEUE {
        assert_ne!(blk.submit(Op::Read, 0, settore(0)).unwrap(), 0);
        blk.process(&mut disco);
        while blk.complete().is_some() {}
    }
}
