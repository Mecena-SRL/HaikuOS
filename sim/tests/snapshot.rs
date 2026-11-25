//! Snapshot copy-on-write: contenuti preservati, costo in blocchi, limiti e rollback.

const KEY: [u8; 16] = *b"chiave-di-prova!";

use haiku_alloc_sim::snapshot::{SnapError, Store, MAX_SNAPSHOTS, SNAP_BLOCKS, SNAP_MAX_FILE};

#[test]
fn snapshot_preserves_old_content_and_rolls_back() {
    let mut s = Store::new(KEY);
    s.write("a", b"v1").unwrap();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    assert_eq!(s.read("a").unwrap(), b"v2");
    s.rollback("s").unwrap();
    assert_eq!(s.read("a").unwrap(), b"v1");
}

#[test]
fn snapshot_costs_no_blocks() {
    let mut s = Store::new(KEY);
    for i in 0..10 {
        s.write(&format!("f{i}"), b"dati").unwrap();
    }
    let before = s.free_blocks();
    s.snapshot("s").unwrap();
    assert_eq!(s.free_blocks(), before);
}

#[test]
fn overwrite_after_snapshot_uses_one_block() {
    let mut s = Store::new(KEY);
    s.write("a", b"v1").unwrap();
    let before = s.free_blocks();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    assert_eq!(s.free_blocks(), before - 1);
}

#[test]
fn dropping_snapshot_frees_its_blocks() {
    let mut s = Store::new(KEY);
    s.write("a", b"v1").unwrap();
    let before = s.free_blocks();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    s.drop_snapshot("s").unwrap();
    assert_eq!(s.free_blocks(), before);
    assert_eq!(s.read("a").unwrap(), b"v2");
}

#[test]
fn deleted_file_comes_back_on_rollback() {
    let mut s = Store::new(KEY);
    s.write("a", b"tenuto").unwrap();
    s.snapshot("s").unwrap();
    assert!(s.delete("a"));
    assert!(s.read("a").is_none());
    s.rollback("s").unwrap();
    assert_eq!(s.read("a").unwrap(), b"tenuto");
}

#[test]
fn rollback_keeps_snapshot_usable() {
    let mut s = Store::new(KEY);
    s.write("a", b"v1").unwrap();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    s.rollback("s").unwrap();
    s.write("a", b"v3").unwrap();
    s.rollback("s").unwrap();
    assert_eq!(s.read("a").unwrap(), b"v1");
}

#[test]
fn no_space_when_all_blocks_used() {
    let mut s = Store::new(KEY);
    for i in 0..SNAP_BLOCKS {
        s.write(&format!("f{i}"), b"x").unwrap();
    }
    assert_eq!(s.write("extra", b"x"), Err(SnapError::NoSpace));
}

#[test]
fn snapshots_hold_blocks_until_dropped() {
    let mut s = Store::new(KEY);
    for i in 0..SNAP_BLOCKS - 1 {
        s.write(&format!("f{i}"), b"x").unwrap();
    }
    s.snapshot("s").unwrap();
    // Un solo blocco libero: l'overwrite di f0 lo usa, il vecchio resta allo snapshot.
    s.write("f0", b"y").unwrap();
    assert_eq!(s.read("f0").unwrap(), b"y");
    assert_eq!(s.write("f1", b"y"), Err(SnapError::NoSpace));
}

#[test]
fn limits_are_enforced() {
    let mut s = Store::new(KEY);
    assert_eq!(s.write("big", &[0; SNAP_MAX_FILE + 1]), Err(SnapError::TooLarge));
    for i in 0..MAX_SNAPSHOTS {
        s.snapshot(&format!("s{i}")).unwrap();
    }
    assert_eq!(s.snapshot("oltre"), Err(SnapError::TooManySnapshots));
    assert_eq!(s.rollback("nessuno"), Err(SnapError::NoSuchSnapshot));
    assert_eq!(s.drop_snapshot("nessuno"), Err(SnapError::NoSuchSnapshot));
}

#[test]
fn tampered_metadata_is_detected() {
    let mut s = Store::new(KEY);
    s.write("a", b"dati").unwrap();
    assert_eq!(s.read_checked("a").unwrap().unwrap(), b"dati");
    s.tamper_meta("a", 5);
    assert_eq!(s.read_checked("a"), Err(SnapError::Corrupt));
    assert!(s.read_checked("nessuno").unwrap().is_none());
}

#[test]
fn tampered_snapshot_blocks_rollback_without_changes() {
    let mut s = Store::new(KEY);
    s.write("a", b"v1").unwrap();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    assert!(s.verify().is_ok());
    s.tamper_snapshot("s", "a", 9);
    assert_eq!(s.verify(), Err(SnapError::Corrupt));
    assert_eq!(s.rollback("s"), Err(SnapError::Corrupt));
    assert_eq!(s.read("a").unwrap(), b"v2");
}

#[test]
fn compressible_file_uses_one_block_and_round_trips() {
    let mut s = Store::new(KEY);
    let data = vec![b'a'; 200];
    let before = s.free_blocks();
    s.write("rep", &data).unwrap();
    assert_eq!(s.free_blocks(), before - 1);
    assert_eq!(s.read_checked("rep").unwrap().unwrap(), data);
}

#[test]
fn incompressible_file_over_a_block_is_refused() {
    let mut s = Store::new(KEY);
    let mut x: u64 = 0x9E37_79B9;
    let noise: Vec<u8> = (0..200).map(|_| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x as u8 }).collect();
    assert_eq!(s.write("noise", &noise), Err(SnapError::TooLarge));
    assert_eq!(s.free_blocks(), SNAP_BLOCKS);
    s.write("small", &noise[..60]).unwrap();
    assert_eq!(s.read("small").unwrap(), &noise[..60]);
}


#[test]
fn flipped_cipher_byte_is_detected_on_read() {
    let mut s = Store::new(KEY);
    s.write("c", b"segreto").unwrap();
    let blk = s.files_block("c");
    s.flip_block_byte(blk);
    s.flip_mirror_byte(blk);
    assert_eq!(s.read_checked("c"), Err(SnapError::Corrupt));
}

#[test]
fn writes_spread_over_blocks_by_wear() {
    let mut s = Store::new(KEY);
    for _ in 0..SNAP_BLOCKS {
        s.write("x", b"v").unwrap();
        s.delete("x");
    }
    for b in 0..SNAP_BLOCKS {
        assert_eq!(s.wear(b), 1);
    }
}

#[test]
fn worn_out_blocks_are_not_reused() {
    let mut s = Store::with_endurance(KEY, 1);
    for i in 0..SNAP_BLOCKS {
        s.write(&format!("f{i}"), b"x").unwrap();
    }
    s.delete("f0");
    assert_eq!(s.write("n", b"x"), Err(SnapError::Worn));
}

#[test]
fn mirror_serves_read_and_scrub_repairs_primary() {
    let mut s = Store::new(KEY);
    s.write("m", b"copia").unwrap();
    let blk = s.files_block("m");
    s.flip_block_byte(blk);
    assert_eq!(s.read_checked("m").unwrap().unwrap(), b"copia");
    assert_eq!(s.scrub(), Ok(1));
    assert_eq!(s.scrub(), Ok(0));
}

#[test]
fn both_copies_lost_is_reported_by_scrub() {
    let mut s = Store::new(KEY);
    s.write("m", b"perso").unwrap();
    let blk = s.files_block("m");
    s.flip_block_byte(blk);
    s.flip_mirror_byte(blk);
    assert_eq!(s.scrub(), Err(SnapError::Corrupt));
}
