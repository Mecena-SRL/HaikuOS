//! Snapshot copy-on-write: contenuti preservati, costo in blocchi, limiti e rollback.

use haiku_alloc_sim::snapshot::{SnapError, Store, MAX_SNAPSHOTS, SNAP_BLOCKS, SNAP_BLOCK_BYTES};

#[test]
fn snapshot_preserves_old_content_and_rolls_back() {
    let mut s = Store::new();
    s.write("a", b"v1").unwrap();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    assert_eq!(s.read("a").unwrap(), b"v2");
    s.rollback("s").unwrap();
    assert_eq!(s.read("a").unwrap(), b"v1");
}

#[test]
fn snapshot_costs_no_blocks() {
    let mut s = Store::new();
    for i in 0..10 {
        s.write(&format!("f{i}"), b"dati").unwrap();
    }
    let before = s.free_blocks();
    s.snapshot("s").unwrap();
    assert_eq!(s.free_blocks(), before);
}

#[test]
fn overwrite_after_snapshot_uses_one_block() {
    let mut s = Store::new();
    s.write("a", b"v1").unwrap();
    let before = s.free_blocks();
    s.snapshot("s").unwrap();
    s.write("a", b"v2").unwrap();
    assert_eq!(s.free_blocks(), before - 1);
}

#[test]
fn dropping_snapshot_frees_its_blocks() {
    let mut s = Store::new();
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
    let mut s = Store::new();
    s.write("a", b"tenuto").unwrap();
    s.snapshot("s").unwrap();
    assert!(s.delete("a"));
    assert!(s.read("a").is_none());
    s.rollback("s").unwrap();
    assert_eq!(s.read("a").unwrap(), b"tenuto");
}

#[test]
fn rollback_keeps_snapshot_usable() {
    let mut s = Store::new();
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
    let mut s = Store::new();
    for i in 0..SNAP_BLOCKS {
        s.write(&format!("f{i}"), b"x").unwrap();
    }
    assert_eq!(s.write("extra", b"x"), Err(SnapError::NoSpace));
}

#[test]
fn snapshots_hold_blocks_until_dropped() {
    let mut s = Store::new();
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
    let mut s = Store::new();
    assert_eq!(s.write("big", &[0; SNAP_BLOCK_BYTES + 1]), Err(SnapError::TooLarge));
    for i in 0..MAX_SNAPSHOTS {
        s.snapshot(&format!("s{i}")).unwrap();
    }
    assert_eq!(s.snapshot("oltre"), Err(SnapError::TooManySnapshots));
    assert_eq!(s.rollback("nessuno"), Err(SnapError::NoSuchSnapshot));
    assert_eq!(s.drop_snapshot("nessuno"), Err(SnapError::NoSuchSnapshot));
}
