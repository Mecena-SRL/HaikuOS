//! Utility di sistema: logica pura, senza allocazioni (`core` soltanto).
//! Disco, info di sistema, coda di stampa, nota MIDI, colorimetro, miniatura dello schermo.
//! Ogni riga di output sta in `LINE` byte (come le righe del terminale). Un comando dà due righe al massimo.
//! Da verificare su hardware: lettura del sensore di colore e cattura dello schermo in ROM (qui c'è solo la logica).

/// Lunghezza massima di una riga (come `terminal::LINE_MAX`).
pub const LINE: usize = 14;
pub const SHOT_W: usize = 240;
pub const SHOT_H: usize = 160;
/// Miniatura 1 bit per pixel: 4800 B. Un'immagine piena (75 KB) non entra nello store da 8 KB.
pub const THUMB_BYTES: usize = SHOT_W * SHOT_H / 8;
/// Lavori in coda per la stampante.
pub const JOBS: u8 = 4;

/// Una riga di testo di lunghezza fissa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    buf: [u8; LINE],
    len: usize,
}

impl Line {
    pub const fn new() -> Self {
        Line { buf: [b' '; LINE], len: 0 }
    }

    /// Aggiunge testo; oltre `LINE` taglia.
    pub fn push(mut self, s: &[u8]) -> Self {
        for &c in s {
            if self.len < LINE {
                self.buf[self.len] = c;
                self.len += 1;
            }
        }
        self
    }

    /// Aggiunge un numero decimale.
    pub fn num(self, n: usize) -> Self {
        let mut d = [0u8; 20];
        let mut i = d.len();
        let mut n = n;
        loop {
            i -= 1;
            d[i] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        self.push(&d[i..])
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Uscita di un comando: una riga, o due (la seconda vuota se non serve).
pub type Out = (Line, Line);

/// Dati del sistema, letti dal chiamante (store del file system e memoria del monolite).
/// Sul GBA: `free_blocks` da `Fs::free_blocks()`, `total_blocks`/`block` da `fs::BLOCKS`/`fs::BLOCK`,
/// `ram`/`pool` da `monolith::RAM_BYTES`/`POOL_BYTES`.
pub struct Env {
    pub free_blocks: usize,
    pub total_blocks: usize,
    pub block: usize,
    pub ram: usize,
    pub pool: usize,
}

/// Disco: percentuale occupata, poi byte usati su totali.
pub fn disk(env: &Env) -> Out {
    let used = env.total_blocks.saturating_sub(env.free_blocks);
    let pct = if env.total_blocks == 0 { 0 } else { used * 100 / env.total_blocks };
    (
        Line::new().push(b"DISCO ").num(pct).push(b"%"),
        Line::new().num(used * env.block).push(b"/").num(env.total_blocks * env.block).push(b" B"),
    )
}

/// Info: memoria totale e del pool, in KB.
pub fn info(env: &Env) -> Out {
    (
        Line::new().push(b"RAM ").num(env.ram / 1024).push(b"K"),
        Line::new().push(b"POOL ").num(env.pool / 1024).push(b"K"),
    )
}

/// Stampante: coda di lavori (un lavoro è `submit`, `finish` lo completa). Stato del terminale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spool {
    pending: u8,
    printed: usize,
}

impl Spool {
    pub const fn new() -> Self {
        Spool { pending: 0, printed: 0 }
    }

    /// Aggiunge un lavoro. Falso se la coda è piena.
    pub fn submit(&mut self) -> bool {
        if self.pending < JOBS {
            self.pending += 1;
            true
        } else {
            false
        }
    }

    /// Completa il lavoro più vecchio. Falso se la coda è vuota.
    pub fn finish(&mut self) -> bool {
        if self.pending > 0 {
            self.pending -= 1;
            self.printed += 1;
            true
        } else {
            false
        }
    }

    pub fn pending(&self) -> u8 {
        self.pending
    }

    pub fn out(&self) -> Out {
        (
            Line::new().push(b"CODA ").num(self.pending as usize).push(b"/").num(JOBS as usize),
            Line::new().push(b"STAMPATE ").num(self.printed),
        )
    }
}

/// Luminosità di un pixel RGB555 (0..=310, pesi 3/6/1 su R/G/B).
pub fn luma(c: u16) -> u32 {
    let r = (c & 31) as u32;
    let g = ((c >> 5) & 31) as u32;
    let b = ((c >> 10) & 31) as u32;
    r * 3 + g * 6 + b
}

/// Miniatura 1 bit: 1 = pixel chiaro. Scrive `THUMB_BYTES` byte in `out`. Falso se le misure non tornano.
pub fn thumb(px: &[u16], out: &mut [u8]) -> bool {
    if px.len() < SHOT_W * SHOT_H || out.len() < THUMB_BYTES {
        return false;
    }
    out[..THUMB_BYTES].fill(0);
    for i in 0..SHOT_W * SHOT_H {
        if luma(px[i]) >= 155 {
            out[i / 8] |= 1 << (i % 8);
        }
    }
    true
}

/// Nome di file di una cattura: SHOT0001.B1 (n con quattro cifre).
pub fn shot_name(n: usize) -> Line {
    let n = n % 10_000;
    Line::new()
        .push(b"SHOT")
        .push(&[b'0' + (n / 1000) as u8, b'0' + (n / 100 % 10) as u8, b'0' + (n / 10 % 10) as u8, b'0' + (n % 10) as u8])
        .push(b".B1")
}

// Frequenze della quarta ottava (C4..B4) in decimi di Hz, intere.
const BASE: [u32; 12] = [2616, 2772, 2937, 3111, 3296, 3492, 3700, 3920, 4153, 4400, 4661, 4937];
const NOTE: [&[u8]; 12] = [b"C", b"C#", b"D", b"D#", b"E", b"F", b"F#", b"G", b"G#", b"A", b"A#", b"B"];

/// Frequenza della nota MIDI `note` (69 = A4 = 440 Hz), in decimi di Hz.
pub fn hz10(note: u8) -> u32 {
    let n = note.min(127) as i32;
    let shift = n / 12 - 1 - 4;
    let base = BASE[(n % 12) as usize];
    if shift >= 0 {
        base << shift as u32
    } else {
        base >> (-shift) as u32
    }
}

/// MIDI: nome della nota e frequenza, es. "A4 440.0 HZ".
pub fn note(n: u8) -> Out {
    let n = n.min(127) as usize;
    let oct = (n / 12) as isize - 1;
    let hz = hz10(n as u8);
    let l = Line::new().push(NOTE[n % 12]);
    let l = if oct < 0 { l.push(b"-1") } else { l.num(oct as usize) };
    (l.push(b" ").num((hz / 10) as usize).push(b".").num((hz % 10) as usize).push(b" HZ"), Line::new())
}

// Colori di riferimento per il nome più vicino.
const REF: [(&[u8], [u8; 3]); 9] = [
    (b"NERO", [0, 0, 0]),
    (b"BIANCO", [255, 255, 255]),
    (b"GRIGIO", [128, 128, 128]),
    (b"ROSSO", [255, 0, 0]),
    (b"VERDE", [0, 255, 0]),
    (b"BLU", [0, 0, 255]),
    (b"GIALLO", [255, 255, 0]),
    (b"CIANO", [0, 255, 255]),
    (b"MAGENTA", [255, 0, 255]),
];

/// Nome del colore di riferimento più vicino (distanza al quadrato in RGB).
pub fn colour_name(c: [u8; 3]) -> &'static [u8] {
    let mut best = REF[0].0;
    let mut best_d = u32::MAX;
    for &(name, r) in REF.iter() {
        let mut d = 0u32;
        for i in 0..3 {
            let a = c[i] as i32 - r[i] as i32;
            d += (a * a) as u32;
        }
        if d < best_d {
            best_d = d;
            best = name;
        }
    }
    best
}

/// Colorimetro: nome del colore e valore esadecimale (due righe).
pub fn colour(c: [u8; 3]) -> Out {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut h = Line::new().push(b"#");
    for v in c {
        h = h.push(&[HEX[(v >> 4) as usize], HEX[(v & 15) as usize]]);
    }
    (Line::new().push(colour_name(c)), h)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Colore da sei cifre esadecimali: "FF8800".
pub fn parse_rgb(s: &[u8]) -> Option<[u8; 3]> {
    if s.len() != 6 {
        return None;
    }
    let mut out = [0u8; 3];
    for i in 0..3 {
        out[i] = hex_val(s[2 * i])? * 16 + hex_val(s[2 * i + 1])?;
    }
    Some(out)
}

/// Numero decimale da 0 a 255 (per le note MIDI).
pub fn parse_u8(s: &[u8]) -> Option<u8> {
    if s.is_empty() {
        return None;
    }
    let mut v: u32 = 0;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v * 10 + (c - b'0') as u32;
        if v > 255 {
            return None;
        }
    }
    Some(v as u8)
}

fn trim(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&c| c != b' ').unwrap_or(s.len());
    let end = s.iter().rposition(|&c| c != b' ').map_or(start, |i| i + 1);
    &s[start..end]
}

/// Comando del terminale (senza distinguere maiuscole). `None` se non è un comando di utility.
/// DISCO, INFO, STAMPA [FINE], NOTA n, COLORE rrggbb.
pub fn command(line: &[u8], env: &Env, spool: &mut Spool) -> Option<Out> {
    let line = trim(line);
    let (cmd, arg) = match line.iter().position(|&c| c == b' ') {
        Some(i) => (&line[..i], trim(&line[i + 1..])),
        None => (line, &line[line.len()..]),
    };
    if cmd.eq_ignore_ascii_case(b"DISCO") {
        Some(disk(env))
    } else if cmd.eq_ignore_ascii_case(b"INFO") {
        Some(info(env))
    } else if cmd.eq_ignore_ascii_case(b"STAMPA") {
        if arg.eq_ignore_ascii_case(b"FINE") {
            spool.finish();
        } else {
            spool.submit();
        }
        Some(spool.out())
    } else if cmd.eq_ignore_ascii_case(b"NOTA") {
        parse_u8(arg).map(note)
    } else if cmd.eq_ignore_ascii_case(b"COLORE") {
        parse_rgb(arg).map(colour)
    } else {
        None
    }
}
