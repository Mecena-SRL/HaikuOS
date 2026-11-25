//! Misura (non un test di regressione): costo di cifratura Ascon-AEAD128 + LZSS sull'host.
//! `cd sim && cargo test --release --test ascon_misura -- --ignored --nocapture`
use haiku_alloc_sim::crypto::Vault;
use std::time::Instant;

#[test]
#[ignore]
fn misura_cifratura_ascon() {
    let key = [7u8; 16];
    for &kb in &[1usize, 4, 8] {
        let plain: Vec<u8> = (0..kb * 1024).map(|i| (i % 251) as u8).collect();
        let mut v = Vault::new(key, 1, 16 * 1024);
        let rounds = 200;
        let t = Instant::now();
        let mut sealed = Vec::new();
        for _ in 0..rounds {
            sealed = v.seal(0, &plain).expect("seal");
        }
        let seal_us = t.elapsed().as_secs_f64() * 1e6 / rounds as f64;
        let t = Instant::now();
        for _ in 0..rounds {
            let _ = v.open(0, &sealed).expect("open");
        }
        let open_us = t.elapsed().as_secs_f64() * 1e6 / rounds as f64;
        println!("{kb} KB: seal {seal_us:.1} us, open {open_us:.1} us, cifrato {} B", sealed.len());
    }
}
