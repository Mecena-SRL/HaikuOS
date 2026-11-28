//! Rendering grafico su framebuffer RGB. Legge soltanto lo stato di `Game`.
//! Nessuna dipendenza esterna: l'output è un PPM, leggibile da qualsiasi visualizzatore.

use crate::{Building, Game, Terrain, WIN_POP};
use std::io::{self, Write};

pub const TILE: usize = 16;
pub const HUD_H: usize = 64;

pub type Rgb = [u8; 3];

pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Frame {
    pub fn new(w: usize, h: usize) -> Self {
        Frame {
            w,
            h,
            px: vec![[0, 0, 0]; w * h],
        }
    }

    /// Riempie un rettangolo, ritagliato ai bordi del framebuffer.
    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, c: Rgb) {
        for yy in y..(y + h).min(self.h) {
            for xx in x..(x + w).min(self.w) {
                self.px[yy * self.w + xx] = c;
            }
        }
    }

    pub fn write_ppm(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.w, self.h)?;
        for p in &self.px {
            out.write_all(p)?;
        }
        Ok(())
    }
}

fn terrain_color(t: Terrain) -> Rgb {
    match t {
        Terrain::Grass => [80, 160, 70],
        Terrain::Forest => [30, 100, 40],
        Terrain::Mountain => [120, 120, 125],
        Terrain::Water => [50, 90, 200],
    }
}

fn building_color(b: Building) -> Rgb {
    match b {
        Building::House => [200, 140, 90],
        Building::Farm => [230, 210, 80],
        Building::Lumber => [110, 70, 30],
        Building::Quarry => [170, 170, 180],
        Building::Market => [240, 120, 200],
        Building::Library => [120, 90, 220],
    }
}

/// Disegna mappa, edifici di entrambi gli imperi e una barra di stato in basso.
pub fn render(g: &Game) -> Frame {
    let (mw, mh) = (g.map.w * TILE, g.map.h * TILE);
    let mut f = Frame::new(mw, mh + HUD_H);
    for y in 0..g.map.h {
        for x in 0..g.map.w {
            let i = g.map.idx(x, y);
            let (px, py) = (x * TILE, y * TILE);
            let t = g.map.terrain(i).map_or([0, 0, 0], terrain_color);
            f.rect(px, py, TILE, TILE, t);
            if let Some(b) = g.player.building_at(i) {
                f.rect(px + 2, py + 2, TILE - 4, TILE - 4, building_color(b));
            }
            if g.rival.building_at(i).is_some() {
                f.rect(px + TILE / 2 - 3, py + TILE / 2 - 3, 6, 6, [220, 30, 30]);
            }
        }
    }
    // Barre HUD: lunghezza proporzionale a ogni risorsa (scala fissa).
    let s = g.player.stock;
    let bars: [(i32, Rgb); 4] = [
        (s.food, [230, 210, 80]),
        (s.wood, [110, 70, 30]),
        (s.stone, [170, 170, 180]),
        (s.gold, [240, 200, 60]),
    ];
    for (row, (v, c)) in bars.iter().enumerate() {
        let len = ((*v).max(0) as usize * 2).min(mw - 8);
        f.rect(4, mh + 6 + row * 12, len, 8, *c);
    }
    let pop = (g.player.citizens as usize * (mw - 8)) / WIN_POP as usize;
    f.rect(4, mh + 6 + 4 * 12 + 4, pop.min(mw - 8), 6, [90, 200, 255]);
    f
}
