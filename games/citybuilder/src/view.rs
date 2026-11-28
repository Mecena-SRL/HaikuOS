//! Rendering vettoriale: mappa isometrica di poligoni, edifici come figure vettoriali, HUD e menu
//! con testo vettoriale (`gui::vfont`). Legge soltanto lo stato di `Game`; nessuna immagine di base.

use crate::menu::Screen;
use crate::raster::{rgb, Frame, Rgb};
use crate::{Building, Game, Terrain, WIN_POP};
use haiku_gui::layout::Rect;
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use haiku_gui::vfont;

const RIVAL: Rgb = [200, 40, 40];
const GRASS: Rgb = [84, 160, 72];
const FOREST: Rgb = [34, 100, 46];
const MOUNTAIN: Rgb = [124, 124, 132];
const WATER: Rgb = [52, 98, 196];
const WHEAT: Rgb = [232, 200, 80];
const SKY_TOP: Rgb = [16, 22, 58];
const SKY_LOW: Rgb = [236, 128, 84];
const EVENING: Rgb = [255, 150, 80];
const SHADOW: Rgb = [0, 0, 0];
const SMOKE: Rgb = [206, 206, 216];
const CURSOR: Rgb = [255, 236, 140];

/// Losanga di una tessera, in unità locali: 100 di larghezza, 50 di altezza, centro in (0, 0).
const TILE: [(f32, f32); 4] = [(0.0, -25.0), (50.0, 0.0), (0.0, 25.0), (-50.0, 0.0)];

/// Sistema locale di una figura: origine al centro della tessera, scala `k` px per unità.
#[derive(Clone, Copy)]
struct Sprite {
    cx: f32,
    cy: f32,
    k: f32,
    tint: Option<Rgb>,
}

impl Sprite {
    fn pt(&self, x: f32, y: f32) -> (f32, f32) {
        (self.cx + x * self.k, self.cy + y * self.k)
    }

    fn col(&self, c: Rgb) -> Rgb {
        match self.tint {
            Some(t) => mix(c, t, 0.45),
            None => c,
        }
    }

    fn poly(&self, f: &mut Frame, pts: &[(f32, f32)], c: Rgb) {
        self.poly_a(f, pts, c, 1.0);
    }

    fn poly_a(&self, f: &mut Frame, pts: &[(f32, f32)], c: Rgb, a: f32) {
        let v: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        f.poly_a(&v, self.col(c), a);
    }

    fn disc(&self, f: &mut Frame, x: f32, y: f32, r: f32, c: Rgb, a: f32) {
        let (px, py) = self.pt(x, y);
        f.disc(px, py, r * self.k, c, a);
    }

    fn rect(&self, f: &mut Frame, x0: f32, y0: f32, x1: f32, y1: f32, c: Rgb) {
        self.poly(f, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], c);
    }

    fn circle(&self, f: &mut Frame, x: f32, y: f32, r: f32, c: Rgb) {
        let (px, py) = self.pt(x, y);
        f.circle(px, py, r * self.k, self.col(c));
    }
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    core::array::from_fn(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t) as u8)
}

fn shade(c: Rgb, d: i32) -> Rgb {
    c.map(|v| (v as i32 + d).clamp(0, 255) as u8)
}

fn terrain(f: &mut Frame, s: &Sprite, t: Terrain, d: i32) {
    match t {
        Terrain::Grass => s.poly(f, &TILE, shade(GRASS, d)),
        Terrain::Water => {
            s.poly(f, &TILE, shade(WATER, d));
            s.rect(f, -26.0, -6.0, -10.0, -4.5, [120, 170, 240]);
            s.rect(f, 8.0, 4.0, 24.0, 5.5, [120, 170, 240]);
        }
        Terrain::Forest => {
            s.poly(f, &TILE, shade(FOREST, d));
            for (x, y) in [(-22.0, -4.0), (8.0, -2.0), (-6.0, 8.0), (22.0, 6.0)] {
                tree(f, s, x, y);
            }
        }
        Terrain::Mountain => {
            s.poly(f, &TILE, shade(MOUNTAIN, d));
            s.poly(
                f,
                &[(-34.0, 6.0), (0.0, -30.0), (34.0, 6.0)],
                [150, 150, 160],
            );
            s.poly(f, &[(0.0, -30.0), (34.0, 6.0), (0.0, 8.0)], [100, 100, 110]);
            s.poly(
                f,
                &[
                    (-9.0, -14.0),
                    (0.0, -30.0),
                    (9.0, -14.0),
                    (4.0, -12.0),
                    (0.0, -17.0),
                    (-4.0, -12.0),
                ],
                [245, 245, 250],
            );
        }
    }
}

fn tree(f: &mut Frame, s: &Sprite, x: f32, y: f32) {
    s.rect(f, x - 2.0, y - 4.0, x + 2.0, y, [100, 64, 30]);
    s.poly(
        f,
        &[(x - 9.0, y - 4.0), (x + 9.0, y - 4.0), (x, y - 22.0)],
        [30, 120, 50],
    );
    s.poly(
        f,
        &[(x - 7.0, y - 12.0), (x + 7.0, y - 12.0), (x, y - 28.0)],
        [44, 140, 60],
    );
}

fn building(f: &mut Frame, s: &Sprite, b: Building) {
    match b {
        Building::House => {
            s.rect(f, -18.0, -22.0, 18.0, 0.0, [214, 170, 120]);
            s.poly(
                f,
                &[(-24.0, -21.0), (24.0, -21.0), (0.0, -44.0)],
                [170, 60, 50],
            );
            s.rect(f, -5.0, -12.0, 5.0, 0.0, [90, 50, 30]);
            s.rect(f, 8.0, -16.0, 14.0, -10.0, [255, 230, 120]);
        }
        Building::Farm => {
            s.rect(f, -28.0, 2.0, 28.0, 4.0, WHEAT);
            s.rect(f, -20.0, 8.0, 20.0, 10.0, WHEAT);
            s.rect(f, -16.0, -22.0, 14.0, -2.0, [170, 56, 44]);
            s.poly(
                f,
                &[(-20.0, -20.0), (18.0, -20.0), (-1.0, -38.0)],
                [240, 236, 226],
            );
            s.rect(f, -6.0, -12.0, 4.0, -2.0, [90, 40, 30]);
        }
        Building::Lumber => {
            s.rect(f, -18.0, -20.0, 16.0, -2.0, [138, 92, 48]);
            s.poly(
                f,
                &[(-22.0, -18.0), (20.0, -18.0), (-1.0, -34.0)],
                [96, 60, 30],
            );
            for (x, y) in [(22.0, -5.0), (30.0, -5.0), (26.0, -11.0)] {
                s.circle(f, x, y, 4.0, [196, 150, 90]);
            }
        }
        Building::Quarry => {
            s.poly(
                f,
                &[(-38.0, 0.0), (0.0, -10.0), (38.0, 0.0), (0.0, 10.0)],
                [70, 70, 78],
            );
            s.poly(
                f,
                &[(-22.0, -2.0), (-12.0, -20.0), (4.0, -22.0), (14.0, -4.0)],
                [150, 150, 160],
            );
            s.poly(
                f,
                &[(-4.0, -4.0), (6.0, -24.0), (22.0, -18.0), (26.0, -2.0)],
                [118, 118, 128],
            );
        }
        Building::Market => {
            s.rect(f, -20.0, -18.0, 20.0, -2.0, [236, 206, 140]);
            for i in 0..5 {
                let x0 = -24.0 + i as f32 * 9.6;
                let c = if i % 2 == 0 {
                    [236, 110, 190]
                } else {
                    [250, 250, 250]
                };
                s.poly(
                    f,
                    &[
                        (x0, -26.0),
                        (x0 + 9.6, -26.0),
                        (x0 + 9.6, -20.0),
                        (x0, -20.0),
                    ],
                    c,
                );
            }
            s.rect(f, -20.0, -8.0, 20.0, -2.0, [150, 100, 60]);
        }
        Building::Library => {
            s.rect(f, -22.0, -4.0, 22.0, 0.0, [170, 170, 180]);
            for i in 0..4 {
                let x = -16.0 + i as f32 * 10.6;
                s.rect(f, x, -26.0, x + 5.0, -4.0, [226, 226, 234]);
            }
            s.poly(
                f,
                &[(-24.0, -26.0), (24.0, -26.0), (0.0, -40.0)],
                [200, 190, 230],
            );
            s.rect(f, -24.0, -26.0, 24.0, -23.0, [120, 90, 220]);
        }
    }
}

/// Ombra morbida a due strati, spostata verso destra in basso (luce da sinistra-alto).
/// Cantiere: piazzola di legno con due montanti e una trave.
fn scaffold(f: &mut Frame, s: &Sprite) {
    let pad: Vec<(f32, f32)> = TILE.iter().map(|&(x, y)| (x * 0.62, y * 0.62)).collect();
    s.poly(f, &pad, [150, 110, 60]);
    for dx in [-18.0f32, 18.0] {
        s.rect(f, dx - 2.0, -34.0, dx + 2.0, -6.0, [110, 80, 40]);
    }
    s.rect(f, -20.0, -34.0, 20.0, -30.0, [200, 160, 90]);
}

fn shadow(f: &mut Frame, s: &Sprite) {
    s.poly_a(
        f,
        &[(-36.0, 8.0), (-6.0, -6.0), (38.0, 4.0), (6.0, 20.0)],
        SHADOW,
        0.12,
    );
    s.poly_a(
        f,
        &[(-26.0, 6.0), (-4.0, -2.0), (30.0, 4.0), (8.0, 14.0)],
        SHADOW,
        0.22,
    );
}

/// Pennacchio di fumo: sfere grigie che salgono e si schiariscono.
fn smoke(f: &mut Frame, s: &Sprite, x: f32, y: f32) {
    for i in 0..4 {
        let fi = i as f32;
        s.disc(
            f,
            x + fi * 3.0,
            y - fi * 11.0,
            3.0 + fi * 2.5,
            SMOKE,
            0.42 - fi * 0.09,
        );
    }
}

/// Icona vettoriale di una risorsa: 0 cibo, 1 legno, 2 pietra, 3 oro, 4 popolazione.
fn icon(f: &mut Frame, kind: usize, cx: f32, cy: f32, r: f32) {
    match kind {
        0 => {
            f.circle(cx, cy, r, [230, 200, 70]);
            f.circle(cx, cy, r * 0.5, [250, 230, 150]);
        }
        1 => {
            f.quad(cx - r, cy - r * 0.4, 2.0 * r, r * 1.3, [140, 90, 40]);
            f.circle(cx - r, cy + r * 0.25, r * 0.4, [200, 150, 90]);
        }
        2 => f.poly(
            &[
                (cx - r, cy + r * 0.6),
                (cx - r * 0.6, cy - r * 0.6),
                (cx + r * 0.4, cy - r),
                (cx + r, cy + r * 0.4),
            ],
            [170, 170, 180],
        ),
        3 => {
            f.circle(cx, cy, r, [255, 200, 40]);
            f.circle(cx, cy, r * 0.6, [255, 236, 140]);
        }
        _ => {
            f.circle(cx, cy - r * 0.5, r * 0.35, [245, 245, 250]);
            f.poly(
                &[
                    (cx - r * 0.7, cy + r),
                    (cx + r * 0.7, cy + r),
                    (cx, cy - r * 0.1),
                ],
                [245, 245, 250],
            );
        }
    }
}

/// Disegna la città (terreno, poi edifici in ordine di profondità) e l'HUD in basso.
/// `w`×`h` è la risoluzione reale: la mappa si adatta alla parte sopra l'HUD.
pub fn render(g: &Game, w: usize, h: usize) -> Frame {
    let (wf, hf) = (w as f32, h as f32);
    let hud = (hf * 0.15).round();
    let scene = hf - hud;
    let mut f = Frame::with_fill(w, h, rgb(Tok::T0));
    f.vgrad(0, scene as usize, SKY_TOP, SKY_LOW);
    let (mw, mh) = (g.map.w as f32, g.map.h as f32);
    // Altezza della mappa: (mw+mh)·tw/4; margine sopra per edifici alti 0.45·tw.
    let tw = (2.0 * wf / (mw + mh)).min(scene / ((mw + mh) / 4.0 + 0.6)) * 0.96;
    let th = tw / 2.0;
    let k = tw / 100.0;
    let ox = (wf - (mw + mh) * tw / 2.0) / 2.0 + mh * tw / 2.0;
    let oy = (scene - (mw + mh) * tw / 4.0) / 2.0 + tw * 0.45;
    // Centro della tessera (x, y): vertice alto della losanga più mezza altezza.
    let centre = |x: usize, y: usize| {
        (
            ox + (x as f32 - y as f32) * tw / 2.0,
            oy + (x as f32 + y as f32) * th / 2.0 + th / 2.0,
        )
    };

    for y in 0..g.map.h {
        for x in 0..g.map.w {
            let i = g.map.idx(x, y);
            let (cx, cy) = centre(x, y);
            let d = ((x * 7 + y * 13) % 9) as i32 - 4;
            let t = g.map.terrain(i).unwrap_or(Terrain::Grass);
            terrain(
                &mut f,
                &Sprite {
                    cx,
                    cy,
                    k,
                    tint: None,
                },
                t,
                d,
            );
        }
    }
    let mut order: Vec<(usize, usize)> = (0..g.map.h)
        .flat_map(|y| (0..g.map.w).map(move |x| (x, y)))
        .collect();
    order.sort_by_key(|&(x, y)| x + y);
    // Prima tutte le ombre (sul terreno), poi edifici in ordine di profondità, poi il fumo.
    for &(x, y) in &order {
        let i = g.map.idx(x, y);
        if g.player.building_at(i).is_some() || g.rival.building_at(i).is_some() {
            let (cx, cy) = centre(x, y);
            shadow(
                &mut f,
                &Sprite {
                    cx,
                    cy,
                    k,
                    tint: None,
                },
            );
        }
    }
    let mut chimneys: Vec<(Sprite, f32, f32)> = Vec::new();
    for &(x, y) in &order {
        let i = g.map.idx(x, y);
        let (cx, cy) = centre(x, y);
        let s = Sprite {
            cx,
            cy,
            k,
            tint: None,
        };
        if g.player.building_at(i).is_none() && g.rival.building_at(i).is_none() {
            if g.player.site_at(i).is_some() || g.rival.site_at(i).is_some() {
                scaffold(&mut f, &s);
            }
            continue;
        }
        let (b, tint) = match (g.player.building_at(i), g.rival.building_at(i)) {
            (Some(b), _) => (b, None),
            (None, Some(b)) => (b, Some(RIVAL)),
            _ => continue,
        };
        let s = Sprite { tint, ..s };
        building(&mut f, &s, b);
        match b {
            Building::House => chimneys.push((s, 10.0, -44.0)),
            Building::Lumber => chimneys.push((s, 0.0, -36.0)),
            Building::Quarry => chimneys.push((s, 10.0, -22.0)),
            _ => {}
        }
    }
    // Cursore del giocatore: contorno della tessera selezionata.
    let (qx, qy) = g.cursor;
    let (cx, cy) = centre(qx, qy);
    let s = Sprite {
        cx,
        cy,
        k,
        tint: None,
    };
    let pts: Vec<(f32, f32)> = TILE.iter().map(|&(x, y)| s.pt(x, y)).collect();
    for j in 0..4 {
        f.line(pts[j], pts[(j + 1) % 4], (3.0 * k).max(2.0), CURSOR);
    }
    for (s, dx, dy) in &chimneys {
        smoke(&mut f, &Sprite { tint: None, ..*s }, *dx, *dy);
    }
    // Luce della sera: velo caldo su tutta la scena (l'HUD viene disegnato dopo).
    f.tint(EVENING, 0.10);

    hud_bar(&mut f, g, scene as i32, hud);
    f
}

fn hud_bar(f: &mut Frame, g: &Game, top: i32, hud: f32) {
    let w = f.w as i32;
    f.fill(Rect::new(0, top, w, hud as i32 + 1), Tok::T0);
    f.fill(
        Rect::new(0, top, w, (hud * 0.02).max(2.0) as i32),
        Tok::A(1),
    );
    let s = g.player.stock;
    let cell = w / 5;
    let label_px = (hud * 0.17).max(10.0) as i32;
    let value_px = (hud * 0.27).max(12.0) as i32;
    let r = hud * 0.2;
    let cells: [(&str, i32, [u8; 3]); 5] = [
        ("CIBO", s.food, [230, 200, 70]),
        ("LEGNO", s.wood, [140, 90, 40]),
        ("PIETRA", s.stone, [170, 170, 180]),
        ("ORO", s.gold, [255, 200, 40]),
        ("POP", g.player.citizens as i32, [245, 245, 250]),
    ];
    for (n, (label, v, c)) in cells.iter().enumerate() {
        let x0 = n as i32 * cell + (hud * 0.25) as i32;
        let cy = top as f32 + hud * 0.42;
        icon(f, n, x0 as f32 + r, cy, r);
        let tx = x0 + (r * 2.0) as i32 + (hud * 0.2) as i32;
        vfont::draw_text(f, tx, top + (hud * 0.12) as i32, label_px, label, Tok::T2);
        let value = if n == 4 {
            format!("{}/{WIN_POP}", v)
        } else {
            v.to_string()
        };
        vfont::draw_text(
            f,
            tx,
            top + (hud * 0.12) as i32 + label_px + 2,
            value_px,
            &value,
            Tok::T1,
        );
        // Barra sotto la cella: pop rispetto all'obiettivo, risorse su scala fissa.
        let frac = if n == 4 {
            *v as f32 / WIN_POP as f32
        } else {
            *v as f32 / 100.0
        };
        let bar_w = cell as f32 * 0.7 * frac.clamp(0.0, 1.0);
        f.quad(
            x0 as f32,
            top as f32 + hud - hud * 0.14,
            bar_w,
            hud * 0.06,
            *c,
        );
    }
    let info = format!("TURNO {}   IA {}", g.turns, g.rival.citizens);
    let size = label_px;
    let adv = vfont::advance(size, &info);
    vfont::draw_text(f, w - adv - 12, 8, size, &info, Tok::T2);
}

/// Sovrappone il menu di avvio o di pausa: fondo scurito, pannello, titolo e voci (la prima selezionata).
pub fn overlay(f: &mut Frame, screen: Screen) {
    if screen == Screen::Play {
        return;
    }
    let scene = (f.h as f32 - (f.h as f32 * 0.15).round()) as usize;
    f.dim_rows(0, scene);
    let (title, rows): (&str, &[&str]) = match screen {
        Screen::Title => ("CITTA IMPERO", &["INIZIA", "ESCI"]),
        _ => ("PAUSA", &["RIPRENDI", "ESCI"]),
    };
    let fs = ((f.h as f32 * 0.06).max(12.0)) as i32;
    let row_h = fs * 2;
    let p = Rect::new(
        f.w as i32 / 4,
        f.h as i32 / 4,
        f.w as i32 / 2,
        fs * 3 + row_h * rows.len() as i32,
    );
    f.fill(p, Tok::T0);
    f.fill(Rect::new(p.x, p.y, p.w, (fs / 4).max(2)), Tok::A(1));
    vfont::draw_text(f, p.x + fs / 2, p.y + fs / 2, fs, title, Tok::A(1));
    for (i, label) in rows.iter().enumerate() {
        let y = p.y + fs * 2 + i as i32 * row_h;
        let (bg, fg) = if i == 0 {
            (Tok::T2, Tok::T0)
        } else {
            (Tok::T0, Tok::T2)
        };
        f.fill(Rect::new(p.x + fs / 4, y, p.w - fs / 2, row_h - fs / 4), bg);
        vfont::draw_text(f, p.x + fs, y + fs / 4, fs, label, fg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scena_vettoriale_disegna_mappa_e_hud() {
        let g = Game::new(24, 12, 7);
        let f = render(&g, 800, 480);
        let fondo = rgb(Tok::T0);
        assert!(f.px.iter().any(|&p| p != fondo));
        // Striscia d'accento sopra l'HUD: prova che l'HUD è stato disegnato.
        assert!(f.px.contains(&rgb(Tok::A(1))));
    }

    /// Costo di un fotogramma: `cargo test --release -- --ignored --nocapture bench`.
    #[test]
    #[ignore]
    fn bench_frame() {
        let g = Game::new(24, 12, 1);
        let t = std::time::Instant::now();
        for _ in 0..20 {
            let mut f = render(&g, 1920, 1080);
            overlay(&mut f, Screen::Play);
            std::hint::black_box(&f);
        }
        println!(
            "BENCH citybuilder {:.1} ms/frame 1920x1080",
            t.elapsed().as_secs_f64() * 1e3 / 20.0
        );
    }
}
