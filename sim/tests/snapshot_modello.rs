//! Test a modello: operazioni casuali deterministiche sullo store, confrontate con un modello semplice.
//! Dopo ogni passo: checksum coerenti e contenuti attesi per vivo e snapshot.

use std::collections::BTreeMap;

use haiku_alloc_sim::snapshot::{SnapError, Store, MAX_SNAPSHOTS, SNAP_BLOCKS};

const KEY: [u8; 16] = *b"chiave-di-prova!";

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn modello_casuale_resta_coerente() {
    let names = ["a", "b", "c", "d"];
    let datas: [&[u8]; 4] = [b"uno", b"due", b"", b"stesso"];
    let mut s = Store::new(KEY);
    let mut live: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut snaps: Vec<(String, BTreeMap<String, Vec<u8>>)> = Vec::new();
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut next_label = 0;

    for _ in 0..3000 {
        match rng.pick(7) {
            0 | 1 => {
                let n = names[rng.pick(names.len())];
                let d = datas[rng.pick(datas.len())];
                match s.write(n, d) {
                    Ok(()) => {
                        live.insert(n.to_string(), d.to_vec());
                    }
                    Err(SnapError::NoSpace) => {}
                    Err(e) => panic!("write: {e:?}"),
                }
            }
            2 => {
                let n = names[rng.pick(names.len())];
                s.delete(n);
                live.remove(n);
            }
            3 => {
                let label = format!("s{next_label}");
                next_label += 1;
                match s.snapshot(&label) {
                    Ok(()) => snaps.push((label, live.clone())),
                    Err(SnapError::TooManySnapshots) => assert_eq!(snaps.len(), MAX_SNAPSHOTS),
                    Err(e) => panic!("snapshot: {e:?}"),
                }
            }
            4 => {
                if !snaps.is_empty() {
                    let (label, m) = snaps[rng.pick(snaps.len())].clone();
                    s.rollback(&label).unwrap();
                    live = m;
                }
            }
            5 => {
                if !snaps.is_empty() {
                    let (label, _) = snaps.remove(rng.pick(snaps.len()));
                    s.drop_snapshot(&label).unwrap();
                }
            }
            _ => {
                let src = names[rng.pick(names.len())];
                let dst = names[rng.pick(names.len())];
                match s.clone_file(src, dst) {
                    Ok(()) => {
                        let d = live.get(src).cloned().expect("clone riuscito senza sorgente");
                        live.insert(dst.to_string(), d);
                    }
                    Err(SnapError::NoSuchFile) => assert!(!live.contains_key(src)),
                    Err(e) => panic!("clone: {e:?}"),
                }
            }
        }
        s.verify().unwrap();
        for n in names {
            assert_eq!(s.read(n), live.get(n).cloned(), "file {n}");
        }
    }

    for (label, _) in snaps.drain(..) {
        s.drop_snapshot(&label).unwrap();
    }
    for n in names {
        s.delete(n);
    }
    assert_eq!(s.free_blocks(), SNAP_BLOCKS);
}
