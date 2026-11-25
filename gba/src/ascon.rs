//! Ascon-AEAD128 (NIST SP 800-232, Ascon v1.2) in software, senza allocazioni.
//! Solo `core`. Niente tabelle: S-box bitsliced e permutazione in cicli semplici su `u64`.
//! Variante identica a `ascon-aead` 0.6 (`AsconAead128`, usato dal simulatore): rate 16 B
//! (due parole), p12 per init/final, p8 per i blocchi dati, parole caricate in little-endian,
//! padding `0x01` seguito da zeri, separazione di dominio `x4 ^= 1 << 63`.
#![allow(dead_code)]

/// IV di Ascon-AEAD128 v1.2: k=128, r=128, a=12, b=8.
const IV: u64 = 0x0000_1000_808c_0001;

/// Permutazione con round da `first` a 11: `first = 0` è p12, `first = 4` è p8.
/// Ogni round: costante, S-box bitsliced (chi + affine), diffusione lineare.
#[inline(always)]
fn perm(s: &mut [u64; 5], first: u32) {
    for r in first..12 {
        s[2] ^= (((0xf - r) << 4) | r) as u64;

        s[0] ^= s[4];
        s[4] ^= s[3];
        s[2] ^= s[1];

        let mut t = [0u64; 5];
        for i in 0..5 {
            t[i] = !s[(i + 1) % 5] & s[(i + 2) % 5];
        }
        for i in 0..5 {
            s[i] ^= t[i];
        }

        s[1] ^= s[0];
        s[0] ^= s[4];
        s[3] ^= s[2];
        s[2] = !s[2];

        s[0] ^= s[0].rotate_right(19) ^ s[0].rotate_right(28);
        s[1] ^= s[1].rotate_right(61) ^ s[1].rotate_right(39);
        s[2] ^= s[2].rotate_right(1) ^ s[2].rotate_right(6);
        s[3] ^= s[3].rotate_right(10) ^ s[3].rotate_right(17);
        s[4] ^= s[4].rotate_right(7) ^ s[4].rotate_right(41);
    }
}

/// Primi 8 byte di `b` come parola little-endian (`b.len() >= 8`).
#[inline(always)]
fn le8(b: &[u8]) -> u64 {
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Byte di `rest` (0..8) in una parola little-endian, zeri in coda.
#[inline(always)]
fn partial(rest: &[u8]) -> u64 {
    let mut b = [0u8; 8];
    b[..rest.len()].copy_from_slice(rest);
    u64::from_le_bytes(b)
}

/// Padding `0x01` subito dopo gli `rest.len()` byte validi.
#[inline(always)]
fn pad(n: usize) -> u64 {
    1u64 << (8 * n)
}

#[inline(always)]
fn key_words(key: &[u8; 16]) -> (u64, u64) {
    (le8(&key[..8]), le8(&key[8..]))
}

/// Inizializzazione: p12(IV || K || N), poi K xor-ato in x3, x4.
fn init(key: &[u8; 16], nonce: &[u8; 16]) -> [u64; 5] {
    let (k0, k1) = key_words(key);
    let mut s = [IV, k0, k1, le8(&nonce[..8]), le8(&nonce[8..])];
    perm(&mut s, 0);
    s[3] ^= k0;
    s[4] ^= k1;
    s
}

/// Dati associati: assorbiti solo se presenti. Poi separazione di dominio (sempre).
fn absorb_ad(s: &mut [u64; 5], ad: &[u8]) {
    if !ad.is_empty() {
        let mut rest = ad;
        while rest.len() >= 16 {
            s[0] ^= le8(rest);
            s[1] ^= le8(&rest[8..]);
            perm(s, 4);
            rest = &rest[16..];
        }
        let sidx = if rest.len() >= 8 {
            s[0] ^= le8(rest);
            rest = &rest[8..];
            1
        } else {
            0
        };
        s[sidx] ^= pad(rest.len()) ^ partial(rest);
        perm(s, 4);
    }
    s[4] ^= 0x8000_0000_0000_0000;
}

/// Cifra o decifra `input` in `out` (stessa lunghezza). Il rate è `s[0], s[1]`.
fn crypt(s: &mut [u64; 5], input: &[u8], out: &mut [u8], decrypt: bool) {
    let n = input.len();
    let mut i = 0;
    while n - i >= 16 {
        for w in 0..2 {
            let off = i + 8 * w;
            // Cifratura: c = rate ^ p. Decifratura: p = rate ^ c. In entrambi il rate diventa il criptogramma.
            let x = le8(&input[off..]);
            let k = x ^ s[w];
            out[off..off + 8].copy_from_slice(&k.to_le_bytes());
            s[w] = if decrypt { x } else { k };
        }
        perm(s, 4);
        i += 16;
    }
    let mut sidx = 0;
    if n - i >= 8 {
        let x = le8(&input[i..]);
        let k = x ^ s[0];
        out[i..i + 8].copy_from_slice(&k.to_le_bytes());
        s[0] = if decrypt { x } else { k };
        i += 8;
        sidx = 1;
    }
    // Ultimo blocco parziale (0..8 byte) con padding.
    let l = n - i;
    let mut sb = s[sidx].to_le_bytes();
    for j in 0..l {
        let inb = input[i + j];
        let ks = sb[j];
        if decrypt {
            out[i + j] = ks ^ inb;
            sb[j] = inb;
        } else {
            let c = ks ^ inb;
            out[i + j] = c;
            sb[j] = c;
        }
    }
    sb[l] ^= 0x01;
    s[sidx] = u64::from_le_bytes(sb);
}

/// Finalizzazione: K in x2, x3, p12, K in x3, x4. Restituisce il tag.
fn finish(s: &mut [u64; 5], key: &[u8; 16]) -> [u8; 16] {
    let (k0, k1) = key_words(key);
    s[2] ^= k0;
    s[3] ^= k1;
    perm(s, 0);
    s[3] ^= k0;
    s[4] ^= k1;
    let mut tag = [0u8; 16];
    tag[..8].copy_from_slice(&s[3].to_le_bytes());
    tag[8..].copy_from_slice(&s[4].to_le_bytes());
    tag
}

/// Cifra `pt` e scrive `ciphertext || tag` in `out`. Ritorna `pt.len() + 16`, oppure 0 se `out` è troppo corto.
pub fn seal(key: &[u8; 16], nonce: &[u8; 16], ad: &[u8], pt: &[u8], out: &mut [u8]) -> usize {
    let need = pt.len() + 16;
    if out.len() < need {
        return 0;
    }
    let mut s = init(key, nonce);
    absorb_ad(&mut s, ad);
    let (ct, rest) = out[..need].split_at_mut(pt.len());
    crypt(&mut s, pt, ct, false);
    let tag = finish(&mut s, key);
    rest.copy_from_slice(&tag);
    need
}

/// Verifica il tag di `ct_tag = ciphertext || tag` e scrive il plaintext in `out`.
/// Ritorna `Some(pt_len)` se il tag è valido. Altrimenti `None`, e `out` viene azzerato.
pub fn open(key: &[u8; 16], nonce: &[u8; 16], ad: &[u8], ct_tag: &[u8], out: &mut [u8]) -> Option<usize> {
    if ct_tag.len() < 16 {
        return None;
    }
    let n = ct_tag.len() - 16;
    if out.len() < n {
        return None;
    }
    let (ct, tag) = ct_tag.split_at(n);
    let mut s = init(key, nonce);
    absorb_ad(&mut s, ad);
    crypt(&mut s, ct, &mut out[..n], true);
    let calc = finish(&mut s, key);
    let mut diff = 0u8;
    for i in 0..16 {
        diff |= calc[i] ^ tag[i];
    }
    if diff != 0 {
        for b in out[..n].iter_mut() {
            *b = 0;
        }
        return None;
    }
    Some(n)
}

/// Nonce di un blocco: gen LE ++ file LE ++ block LE ++ 4 zeri.
#[inline(always)]
fn block_nonce(gen: u32, file: u32, block: u32) -> [u8; 16] {
    let mut n = [0u8; 16];
    n[0..4].copy_from_slice(&gen.to_le_bytes());
    n[4..8].copy_from_slice(&file.to_le_bytes());
    n[8..12].copy_from_slice(&block.to_le_bytes());
    n
}

/// Cifra un blocco da 64 B: scrive 64 B di ciphertext seguiti dal tag da 16 B.
pub fn seal_block(key: &[u8; 16], gen: u32, file: u32, block: u32, pt: &[u8; 64], out: &mut [u8; 80]) {
    let nonce = block_nonce(gen, file, block);
    seal(key, &nonce, &[], pt, out);
}

/// Verifica e decifra un blocco da 80 B (64 B ciphertext + tag). `false` se il tag non torna.
pub fn open_block(key: &[u8; 16], gen: u32, file: u32, block: u32, ct: &[u8; 80], out: &mut [u8; 64]) -> bool {
    let nonce = block_nonce(gen, file, block);
    open(key, &nonce, &[], ct, out).is_some()
}
