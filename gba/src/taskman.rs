//! Task manager: elenco dei programmi del monolite, con stato e quota. Sola lettura.
//! Stati: LIBERO (non registrato), REGISTRATO (senza quota), ATTIVO (quota concessa).
#![allow(dead_code)]

use crate::font;
use crate::monolith::{Monolith, MAX_PROGRAMS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Free,
    Registered,
    Active,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Row {
    pub id: usize,
    pub state: State,
    /// Byte concessi (0 se nessuna quota).
    pub bytes: usize,
}

/// Una riga per ogni slot di programma, in ordine.
pub fn rows(m: &Monolith) -> [Row; MAX_PROGRAMS] {
    let mut out = [Row { id: 0, state: State::Free, bytes: 0 }; MAX_PROGRAMS];
    for p in 0..MAX_PROGRAMS {
        let bytes = m.grant_of(p);
        let state = if bytes > 0 {
            State::Active
        } else if m.is_registered(p) {
            State::Registered
        } else {
            State::Free
        };
        out[p] = Row { id: p, state, bytes };
    }
    out
}

/// Testo dello stato, in maiuscolo come il resto del font.
pub fn label(s: State) -> &'static [u8] {
    match s {
        State::Free => b"LIBERO",
        State::Registered => b"REGISTRATO",
        State::Active => b"ATTIVO",
    }
}

/// Riga di testo: "P0 ATTIVO 32K". Ritorna la lunghezza scritta in `out`.
pub fn line(r: &Row, out: &mut [u8; 24]) -> usize {
    let mut n = 0;
    out[n] = b'P';
    out[n + 1] = b'0' + r.id as u8;
    n += 2;
    out[n] = b' ';
    n += 1;
    for &c in label(r.state) {
        out[n] = c;
        n += 1;
    }
    out[n] = b' ';
    n += 1;
    let kb = r.bytes / 1024;
    // Fino a tre cifre (la quota massima è 32 KB).
    let digits = if kb >= 100 { 3 } else if kb >= 10 { 2 } else { 1 };
    let mut v = kb;
    for i in (0..digits).rev() {
        out[n + i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    n += digits;
    out[n] = b'K';
    n + 1
}

/// Disegna l'elenco nel buffer `fb` (largo `w`, alto `h`), una riga per programma a partire da (x, y).
pub fn draw(fb: &mut [u16], w: usize, h: usize, m: &Monolith, x: usize, y: usize, fg: u16) {
    let rs = rows(m);
    for (i, r) in rs.iter().enumerate() {
        let mut buf = [0u8; 24];
        let n = line(r, &mut buf);
        font::draw(fb, w, h, x, y + i * 8, &buf[..n], fg);
    }
}
