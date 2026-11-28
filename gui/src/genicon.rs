//! Icone generative (docs/vector-generative.md). Nessun disegno salvato: ogni icona è una
//! formula sulla cella. Il livello di dettaglio (LOD) dipende dalla dimensione fisica; la
//! cache conserva solo il risultato per dimensione e si sostituisce quando lo schermo cresce.
//! Logica pura, no_std, senza allocazione. Geometria relativa all'origine della cella.
use crate::layout::Rect;
use crate::palette::Tok;

/// Soglie LOD in pixel fisici (lato minore della cella). Sotto 32: LOD0.
pub const LOD1_MIN: i32 = 32;
pub const LOD2_MIN: i32 = 64;
/// Massimo di primitivi per icona (cartella: 4).
pub const MAX_PRIMS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lod {
    L0,
    L1,
    L2,
}

/// Un primitivo: rettangolo con raggio d'angolo (0 = retto) e token colore.
#[derive(Clone, Copy, Debug)]
pub struct Prim {
    pub r: Rect,
    pub radius: i32,
    pub t: Tok,
}

/// Livello di dettaglio per la cella.
pub fn lod_for(cell: Rect) -> Lod {
    let s = cell.w.min(cell.h);
    if s >= LOD2_MIN {
        Lod::L2
    } else if s >= LOD1_MIN {
        Lod::L1
    } else {
        Lod::L0
    }
}

/// Cartella generata nella cella. Griglia 16 (come `icon_vec`): ogni coordinata è k/16 del lato.
/// Gerarchia: retro e linguetta sempre; falda da L1; raggi e luce da L2.
/// Restituisce (numero di primitivi, LOD). I primitivi sono relativi alla cella.
pub fn folder(cell: Rect, out: &mut [Prim; MAX_PRIMS]) -> (usize, Lod) {
    let lod = lod_for(cell);
    let s = cell.w.min(cell.h);
    // Raggio = 1/8 del lato in L2, retto altrimenti.
    let rad = if lod == Lod::L2 { s / 8 } else { 0 };
    let px = |k: i32| cell.x + k * cell.w / 16;
    let py = |k: i32| cell.y + k * cell.h / 16;
    let rect = |x0: i32, y0: i32, x1: i32, y1: i32| Rect::new(px(x0), py(y0), px(x1) - px(x0), py(y1) - py(y0));

    // Raggio mai oltre metà del lato corto della forma (la linguetta è bassa).
    let fit = |r: Rect| rad.min(r.w.min(r.h) / 2);
    let (corpo, linguetta) = (rect(1, 4, 15, 13), rect(1, 2, 7, 5));
    out[0] = Prim { r: corpo, radius: fit(corpo), t: Tok::T1 }; // retro
    out[1] = Prim { r: linguetta, radius: fit(linguetta), t: Tok::T1 }; // linguetta
    let mut n = 2;
    if lod != Lod::L0 {
        let falda = rect(1, 6, 15, 14);
        out[n] = Prim { r: falda, radius: fit(falda), t: Tok::T2 }; // falda
        n += 1;
    }
    if lod == Lod::L2 {
        out[n] = Prim { r: rect(2, 6, 14, 7), radius: 0, t: Tok::T3 }; // luce sul bordo della falda
        n += 1;
    }
    (n, lod)
}

/// Esito di una generazione, salvabile in cache. Coordinate relative all'origine.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub w: i32,
    pub h: i32,
    pub lod: Lod,
    pub n: usize,
    pub prims: [Prim; MAX_PRIMS],
}

/// Genera senza cache.
pub fn generate_folder(w: i32, h: i32) -> Entry {
    let mut prims = [Prim { r: Rect::new(0, 0, 0, 0), radius: 0, t: Tok::T0 }; MAX_PRIMS];
    let (n, lod) = folder(Rect::new(0, 0, w, h), &mut prims);
    Entry { w, h, lod, n, prims }
}

/// Cache a slot fissi: uno per tipo di icona (`kind`). Un cambio di dimensione sostituisce la voce.
pub struct Cache<const N: usize> {
    slots: [Option<Entry>; N],
    pub hits: u32,
    pub misses: u32,
}

impl<const N: usize> Cache<N> {
    pub const fn new() -> Self {
        Cache { slots: [None; N], hits: 0, misses: 0 }
    }

    /// Icona cartella per la dimensione `w`×`h`: dalla cache se la dimensione coincide, altrimenti rigenera.
    pub fn folder(&mut self, kind: usize, w: i32, h: i32) -> Entry {
        let slot = &mut self.slots[kind % N];
        match slot {
            Some(e) if e.w == w && e.h == h => self.hits += 1,
            _ => {
                self.misses += 1;
                *slot = Some(generate_folder(w, h));
            }
        }
        // Lo slot è sempre pieno qui: o era già valido o è stato appena scritto.
        match *slot {
            Some(e) => e,
            None => generate_folder(w, h),
        }
    }
}

/// Unità base `u` derivata dalla superficie: lato minore / 128, minimo 1 px. Interi su schermi piccoli,
/// cresce con la superficie su quelli grandi. Icona = 16 u; padding = 2 u (s/8); spazio = 1 u (s/16).
pub const GRID: i32 = 128;
pub const ICON_U: i32 = 16;

/// Classe di schermo: decide il minimo di leggibilità del testo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// lato minore < 360 px (GBA, PSP 480×272)
    Small,
    /// 360 ≤ lato minore < 1000 px
    Medium,
    /// lato minore ≥ 1000 px (1080p, 4K)
    Large,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Unit {
    pub u: i32,
    pub class: Class,
}

pub fn unit_for(w: i32, h: i32) -> Unit {
    let s = w.min(h);
    let class = if s < 360 {
        Class::Small
    } else if s < 1000 {
        Class::Medium
    } else {
        Class::Large
    };
    Unit { u: (s / GRID).max(1), class }
}

impl Unit {
    /// Lato dell'icona in pixel.
    pub const fn icon(&self) -> i32 {
        ICON_U * self.u
    }
    /// Testo: 2 u, con minimo per classe (Small 8, Medium 10, Large 12 px).
    pub fn text_px(&self) -> i32 {
        let min = match self.class {
            Class::Small => 8,
            Class::Medium => 10,
            Class::Large => 12,
        };
        (2 * self.u).max(min)
    }
}
