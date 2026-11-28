//! Monitor di sistema: istantanea di disco, memoria e stampante in quattro righe.
//! Logica pura, senza allocazioni (`core` soltanto). Riusa `sysutil` e non legge hardware.

use super::sysutil::{disk, info, Env, Line, Spool};

/// Righe dell'istantanea: DISCO n%, RAM, POOL, coda di stampa.
pub const RIGHE: usize = 4;

/// Istantanea: `[DISCO n%, RAM K, POOL K, CODA n/m]`.
pub fn istantanea(env: &Env, spool: &Spool) -> [Line; RIGHE] {
    let (d1, _) = disk(env);
    let (r1, r2) = info(env);
    let (c1, _) = spool.out();
    [d1, r1, r2, c1]
}

#[cfg(test)]
mod test {
    use super::*;

    fn env(libero: usize) -> Env {
        Env { free_blocks: libero, total_blocks: 100, block: 512, ram: 288 * 1024, pool: 32 * 1024 }
    }

    #[test]
    fn istantanea_con_disco_mezzo_pieno() {
        let s = istantanea(&env(50), &Spool::new());
        assert_eq!(s[0].as_bytes(), b"DISCO 50%");
        assert_eq!(s[1].as_bytes(), b"RAM 288K");
        assert_eq!(s[2].as_bytes(), b"POOL 32K");
        assert_eq!(s[3].as_bytes(), b"CODA 0/4");
    }

    #[test]
    fn disco_vuoto_non_divide_per_zero() {
        let e = Env { free_blocks: 0, total_blocks: 0, block: 512, ram: 0, pool: 0 };
        assert_eq!(istantanea(&e, &Spool::new())[0].as_bytes(), b"DISCO 0%");
    }
}
