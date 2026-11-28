//! Rendering grafico su framebuffer RGB. Legge soltanto lo stato di `Game`.
//! Nessuna dipendenza esterna: l'output è un PPM, leggibile da qualsiasi visualizzatore.

use crate::menu::Screen;
use crate::{Building, Game, Terrain, WIN_POP};
use haiku_gui::layout::{Rect, MENU_ROW};
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use std::io::{self, Write};

#[path = "../../../gba/src/font.rs"]
mod font;

/// Ingrandimento del PPM di uscita (il disegno è a risoluzione logica).
pub const SCALE: usize = 2;

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

    /// Scurisce tutto il fotogramma (sfondo del menu).
    pub fn dim(&mut self) {
        for p in &mut self.px {
            *p = p.map(|c| c / 3);
        }
    }

    pub fn write_ppm(&self, out: &mut impl Write) -> io::Result<()> {
        let (w, h) = (self.w * SCALE, self.h * SCALE);
        write!(out, "P6\n{w} {h}\n255\n")?;
        for y in 0..h {
            for x in 0..w {
                out.write_all(&self.px[(y / SCALE) * self.w + x / SCALE])?;
            }
        }
        Ok(())
    }
}

fn rgb(t: Tok) -> Rgb {
    let c = t.rgb();
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

impl Canvas for Frame {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = rgb(t);
        for y in r.y.max(0)..(r.y + r.h).min(self.h as i32) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w as i32) {
                self.px[y as usize * self.w + x as usize] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        for (i, ch) in s.bytes().enumerate() {
            let gx = x + i as i32 * font::ADVANCE as i32;
            for (row, bits) in font::glyph(ch).iter().enumerate() {
                for col in 0..font::GLYPH_W {
                    if (bits >> (font::GLYPH_W - 1 - col)) & 1 == 1 {
                        self.fill(Rect::new(gx + col as i32, y + row as i32, 1, 1), t);
                    }
                }
            }
        }
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
    // HUD: etichetta a sinistra, barra proporzionale a ogni risorsa (scala fissa) a destra.
    const BX: usize = 48;
    let s = g.player.stock;
    let bars: [(&str, i32, Rgb); 4] = [
        ("CIBO", s.food, [230, 210, 80]),
        ("LEGNO", s.wood, [110, 70, 30]),
        ("PIETRA", s.stone, [170, 170, 180]),
        ("ORO", s.gold, [240, 200, 60]),
    ];
    for (row, (name, v, c)) in bars.iter().enumerate() {
        let y = (mh + 6 + row * 12) as i32;
        f.text(4, y + 1, name, Tok::T1);
        let len = ((*v).max(0) as usize * 2).min(mw - BX - 4);
        f.rect(BX, mh + 6 + row * 12, len, 8, *c);
    }
    let y = (mh + 6 + 4 * 12) as i32;
    f.text(4, y + 1, "POP", Tok::T1);
    let pop = (g.player.citizens as usize * (mw - BX - 4)) / WIN_POP as usize;
    f.rect(BX, mh + 6 + 4 * 12 + 2, pop.min(mw - BX - 4), 6, [90, 200, 255]);
    f
}

/// Sovrappone il menu di avvio o di pausa: fondo scurito, pannello, titolo e voci (la prima selezionata).
/// Le voci sono centrate nella riga: `Canvas::menu` di gui mette il testo fuori riga.
pub fn overlay(f: &mut Frame, screen: Screen) {
    if screen == Screen::Play {
        return;
    }
    f.dim();
    let (title, rows): (&str, &[&str]) = match screen {
        Screen::Title => ("CITTA IMPERO", &["INIZIA", "ESCI"]),
        _ => ("PAUSA", &["RIPRENDI", "ESCI"]),
    };
    let p = Rect::new(f.w as i32 / 4, f.h as i32 / 4, f.w as i32 / 2, 22 + MENU_ROW * rows.len() as i32 + 4);
    f.fill(p, Tok::T0);
    f.fill(Rect::new(p.x, p.y, p.w, 3), Tok::A(1));
    f.text(p.x + 6, p.y + 6, title, Tok::A(1));
    for (i, label) in rows.iter().enumerate() {
        let y = p.y + 18 + i as i32 * MENU_ROW;
        let (bg, fg) = if i == 0 { (Tok::T2, Tok::T0) } else { (Tok::T0, Tok::T2) };
        f.fill(Rect::new(p.x + 4, y, p.w - 8, MENU_ROW), bg);
        f.text(p.x + 8, y + 2, label, fg);
    }
}
