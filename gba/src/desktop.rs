//! Schermata del desktop dal concept (docs/desktop-concept.md, sezioni 2 e 3).
//! Quattro tinte retro (docs/desktop-concept.md). Disegna in un framebuffer 240x160, senza dipendenze da hardware.
#![allow(dead_code)]

pub const W: usize = 240;
pub const H: usize = 160;

// Quattro tinte (docs/desktop-concept.md, scelta di Ivan).
pub const T0: u16 = 1 | (2 << 5) | (6 << 10); // sfondo
pub const T1: u16 = 31 | (31 << 5) | (31 << 10); // corpo finestra, icona selezionata
pub const T2: u16 = 31 | (16 << 5); // titolo finestra, icone
pub const T3: u16 = 20 << 5 | (31 << 10); // barra superiore

pub const BG: u16 = T0;
pub const BAR: u16 = T3;
pub const BAR_SLOT: u16 = T2;
pub const CLOCK: u16 = T1;
pub const ICON: u16 = T2;
pub const ICON_HI: u16 = T1;
pub const WIN_TITLE: u16 = T2;
pub const WIN_BODY: u16 = T1;
pub const WIN_BORDER: u16 = T3;
pub const CLOSE: u16 = T1;
pub const TEXT: u16 = T0;

/// Barra superiore, in unità GBA (1 unità = 1 px).
pub const BAR_H: usize = 12;
/// Icone 16x16, griglia con passo 48 px (dal concept).
pub const ICON_SIZE: usize = 16;
pub const ICON_STEP_X: usize = 48;
pub const ICON_STEP_Y: usize = 40;
/// Finestra del terminale: posizione e misure.
pub const WIN_X: usize = 124;
pub const WIN_Y: usize = 30;
pub const WIN_W: usize = 108;
pub const WIN_H: usize = 88;
pub const TITLE_H: usize = 10;
/// Numero di icone sulla riga (4 colonne, una sola riga nel primo schizzo).
pub const ICONS: usize = 8;
/// Etichette sotto le icone (files, term, giochi, notes).
pub const ICON_LABELS: [&[u8]; ICONS] = [b"FILES", b"TERM", b"CALC", b"NOTES", b"CLOCK", b"SETTINGS", b"CHESS", b"READ"];
/// Icona che apre la finestra del terminale.
pub const TERM_ICON: usize = 1;
/// Icona che apre la calcolatrice.
pub const CALC_ICON: usize = 2;
/// Icone degli scacchi e del lettore di testo (finestre 64x48).
pub const CHESS_ICON: usize = 6;
pub const READ_ICON: usize = 7;
/// Testo di prova del lettore (maiuscole: il font ha solo quelle).
const READER_TEXT: &[u8] = b"HAIKUOS E UN SISTEMA PER GBA. QUESTO E UN TESTO DI PROVA PER IL LETTORE DI PAGINE.";

/// Programmi che il desktop sa lanciare. `None` = icona senza programma (ancora da fare).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum App {
    None,
    /// Spazio di lavoro (file manager): schermo intero, si esce con L.
    Files,
    /// Visualizzatore/editor di testo: schermo intero, si esce con Start.
    Notes,
    Term,
    Calc,
    /// Orologio: finestra con ora e regolazione.
    Clock,
    /// Impostazioni: tema e nome del sistema.
    Settings,
    /// Scacchi: finestra 64x48, Select chiude.
    Chess,
    /// Lettore di testo: finestra 64x48, Select chiude.
    Reader,
}

/// Tabella del lanciatore: programma dietro a ogni icona.
pub const APPS: [App; ICONS] = [App::Files, App::Term, App::Calc, App::Notes, App::Clock, App::Settings, App::Chess, App::Reader];

/// Programma lanciabile dalla icona `i`, se c'è una finestra per lui.
pub fn launchable(i: usize) -> Option<App> {
    match APPS.get(i).copied().unwrap_or(App::None) {
        App::None => None,
        a => Some(a),
    }
}

use crate::apps::{clock::Clock, settings::Settings, view};
use crate::media_ui::{self, ChessView, Key, Reader};
use crate::calc_ui::{self, CalcUi};
use crate::fs::Fs;
use crate::terminal::{self, Terminal};

pub const KEY_A: u16 = 1 << 0;
pub const KEY_B: u16 = 1 << 1;
pub const KEY_RIGHT: u16 = 1 << 4;
pub const KEY_LEFT: u16 = 1 << 5;
pub const KEY_UP: u16 = 1 << 6;
pub const KEY_DOWN: u16 = 1 << 7;
pub const KEY_SELECT: u16 = 1 << 2;

/// Stato del desktop: icona selezionata, programma in finestra (indice icona), calcolatrice.
pub struct Desktop {
    sel: usize,
    win: Option<App>,
    pub term: Terminal,
    /// Programma a schermo intero da avviare (lo gestisce il chiamante).
    run: Option<App>,
    prev: u16,
    calc: CalcUi,
    clock: Clock,
    settings: Settings,
    chess: ChessView,
    reader: Reader,
}

impl Desktop {
    pub fn new() -> Self {
        Self { sel: 0, win: None, prev: 0, calc: CalcUi::new(), term: Terminal::new(), run: None, clock: Clock::new(9, 0), settings: Settings::new(), chess: ChessView::new(), reader: Reader::new(READER_TEXT) }
    }

    /// Vero se c'è un programma a schermo intero da avviare.
    pub fn run_requested(&self) -> bool {
        self.run.is_some()
    }

    /// Prende il programma a schermo intero richiesto.
    pub fn take_run(&mut self) -> Option<App> {
        self.run.take()
    }

    pub fn sel(&self) -> usize {
        self.sel
    }

    pub fn win_open(&self) -> bool {
        self.win.is_some()
    }

    pub fn calc(&self) -> &CalcUi {
        &self.calc
    }

    /// Disegna la superficie della calcolatrice (SW * SH pixel).
    pub fn draw_calc(&self, surf: &mut [u16]) {
        self.calc.draw(surf);
    }

    /// Disegna la superficie degli scacchi o del lettore (media_ui::W * H pixel), se aperti.
    pub fn draw_media(&self, surf: &mut [u16]) {
        match self.win {
            Some(App::Chess) => self.chess.draw(surf),
            Some(App::Reader) => self.reader.draw(surf),
            _ => {}
        }
    }

    /// Un passo di input. Con la calcolatrice in finestra, la croce e A vanno a lei; B chiude la finestra.
    /// Ritorna true se il desktop va ridisegnato.
    /// Avanza di un frame l'orologio. Ritorna true se il testo dell'ora è cambiato.
    pub fn tick(&mut self) -> bool {
        let prima = self.clock.testo();
        self.clock.tick(1);
        prima != self.clock.testo()
    }

    pub fn step(&mut self, keys: u16, fs: &Fs) -> bool {
        let pressed = keys & !self.prev;
        self.prev = keys;
        let before = (
            self.sel,
            self.win,
            self.calc.cursor(),
            self.calc.calc().shown(),
            self.calc.calc().err.is_some(),
            self.term.sel,
            self.term.lines(),
            self.term.typing,
            self.term.pick,
            self.term.input().len(),
            self.clock.hms(),
            self.settings.theme(),
        );
        match self.win {
            Some(App::Chess) | Some(App::Reader) => {
                if pressed & KEY_SELECT != 0 {
                    self.win = None;
                }
                let keys_map = [
                    (KEY_UP, Key::Up),
                    (KEY_DOWN, Key::Down),
                    (KEY_LEFT, Key::Left),
                    (KEY_RIGHT, Key::Right),
                    (KEY_A, Key::A),
                    (KEY_B, Key::B),
                ];
                for (bit, k) in keys_map {
                    if pressed & bit == 0 {
                        continue;
                    }
                    if self.win == Some(App::Chess) {
                        self.chess.key(k);
                    } else {
                        match k {
                            Key::Right | Key::Down | Key::A => {
                                self.reader.next();
                            }
                            Key::Left | Key::Up | Key::B => {
                                self.reader.prev();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some(App::Calc) => {
                self.calc.step(keys);
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Term) => {
                if pressed & KEY_SELECT != 0 {
                    self.term.toggle_typing();
                }
                if self.term.typing {
                    // Digitazione: su/giù scelgono il carattere, A lo aggiunge, B cancella, sinistra esegue.
                    if pressed & KEY_UP != 0 {
                        self.term.pick_step(-1);
                    }
                    if pressed & KEY_DOWN != 0 {
                        self.term.pick_step(1);
                    }
                    if pressed & KEY_A != 0 {
                        self.term.type_pick();
                    }
                    if pressed & KEY_B != 0 {
                        self.term.backspace();
                    }
                    if pressed & KEY_LEFT != 0 {
                        self.term.run_input(fs);
                    }
                } else {
                    if pressed & KEY_UP != 0 {
                        self.term.step_sel(fs, -1);
                    }
                    if pressed & KEY_DOWN != 0 {
                        self.term.step_sel(fs, 1);
                    }
                    if pressed & KEY_A != 0 {
                        self.term.run(fs, self.term.sel);
                    }
                    if pressed & KEY_B != 0 {
                        self.win = None;
                    }
                }
            }
            Some(App::Clock) => {
                if pressed & KEY_UP != 0 {
                    self.clock.adjust(1);
                }
                if pressed & KEY_DOWN != 0 {
                    self.clock.adjust(-1);
                }
                if pressed & KEY_RIGHT != 0 {
                    self.clock.adjust(60);
                }
                if pressed & KEY_LEFT != 0 {
                    self.clock.adjust(-60);
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Settings) => {
                if pressed & KEY_RIGHT != 0 {
                    self.settings.next_theme();
                }
                if pressed & KEY_LEFT != 0 {
                    self.settings.prev_theme();
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            _ => {
                if pressed & (KEY_RIGHT | KEY_DOWN) != 0 {
                    self.sel = (self.sel + 1) % ICONS;
                }
                if pressed & (KEY_LEFT | KEY_UP) != 0 {
                    self.sel = (self.sel + ICONS - 1) % ICONS;
                }
                if pressed & KEY_A != 0 {
                    match launchable(self.sel) {
                        Some(App::Files) | Some(App::Notes) => self.run = launchable(self.sel),
                        Some(app) => self.win = Some(app),
                        None => {}
                    }
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
        }
        before
            != (
                self.sel,
                self.win,
                self.calc.cursor(),
                self.calc.calc().shown(),
                self.calc.calc().err.is_some(),
                self.term.sel,
                self.term.lines(),
                self.term.typing,
                self.term.pick,
                self.term.input().len(),
                self.clock.hms(),
                self.settings.theme(),
            )
    }
}

/// Copia `src` in maiuscolo in `dst` (il font ha solo le maiuscole). Ritorna i byte scritti.
fn upper_into(dst: &mut [u8], src: &[u8]) -> usize {
    let n = src.len().min(dst.len());
    for i in 0..n {
        dst[i] = src[i].to_ascii_uppercase();
    }
    n
}

/// Posizione (x, y) dell'icona `i` nella griglia: 4 colonne.
pub fn icon_pos(i: usize) -> (usize, usize) {
    (8 + (i % 4) * ICON_STEP_X, BAR_H + 12 + (i / 4) * ICON_STEP_Y)
}

fn put(fb: &mut [u16], x: usize, y: usize, c: u16) {
    if x < W && y < H {
        fb[y * W + x] = c;
    }
}

fn rect(fb: &mut [u16], x: usize, y: usize, w: usize, h: usize, c: u16) {
    for yy in y..(y + h).min(H) {
        for xx in x..(x + w).min(W) {
            fb[yy * W + xx] = c;
        }
    }
}

/// Disegna il desktop completo in `fb` (W * H pixel). `surf` è la superficie della calcolatrice.
pub fn paint(fb: &mut [u16], d: &Desktop, surf: &[u16], media: &[u16], fs: &Fs) {
    assert!(fb.len() >= W * H);
    let [t0, t1, t2, t3] = d.settings.palette();
    rect(fb, 0, 0, W, H, t0);

    // Barra: nome a sinistra, due slot programma al centro, orologio a destra.
    rect(fb, 0, 0, W, BAR_H, t3);
    crate::font::draw(fb, W, H, 4, 3, b"HAIKUOS", t0);
    crate::font::draw(fb, W, H, 92, 3, b"TERM", t0);
    crate::font::draw(fb, W, H, 120, 3, b"FILES", t0);
    crate::font::draw(fb, W, H, W - 30, 3, &d.clock.testo(), t1);

    // Icone con etichetta. Riquadro chiaro = icona selezionata.
    for i in 0..ICONS {
        let (x, y) = icon_pos(i);
        let c = if i == d.sel() { t1 } else { t2 };
        rect(fb, x, y, ICON_SIZE, ICON_SIZE, c);
        crate::font::draw(fb, W, H, x, y + ICON_SIZE + 2, ICON_LABELS[i], T1);
    }
    let app = match d.win {
        Some(a) => a,
        None => return,
    };

    // Finestra: titolo, corpo con bordo di 1 px, pulsante chiudi 8x8.
    rect(fb, WIN_X, WIN_Y, WIN_W, WIN_H, t3);
    rect(fb, WIN_X + 1, WIN_Y + 1, WIN_W - 2, TITLE_H, t2);
    rect(fb, WIN_X + WIN_W - 9, WIN_Y + 1, 8, 8, t1);
    rect(fb, WIN_X + 1, WIN_Y + TITLE_H + 1, WIN_W - 2, WIN_H - TITLE_H - 2, t1);
    let bx = WIN_X + 1;
    let by = WIN_Y + TITLE_H + 1;
    if app == App::Chess || app == App::Reader {
        let title: &[u8] = if app == App::Chess { b"CHESS" } else { b"READ" };
        crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, title, t0);
        // Superficie 64x48 centrata nel corpo della finestra.
        let ox = bx + (WIN_W - 2 - media_ui::W) / 2;
        let oy = by + (WIN_H - TITLE_H - 2 - media_ui::H) / 2;
        for y in 0..media_ui::H {
            for x in 0..media_ui::W {
                fb[(oy + y) * W + ox + x] = media[y * media_ui::W + x];
            }
        }
        return;
    }
    if matches!(app, App::Clock | App::Settings) {
        let nome: &[u8] = if app == App::Clock { b"ORA" } else { b"IMPOSTAZIONI" };
        crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, nome, t0);
        match app {
            App::Clock => view::clock(fb, W, H, bx, by, &d.clock, d.settings.palette()),
            _ => view::settings(fb, W, H, bx, by, &d.settings, d.settings.palette()),
        }
        return;
    }
    if app == App::Calc {
        crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, b"CALC", t0);
        // Superficie propria della calcolatrice, copiata nel corpo della finestra.
        for y in 0..calc_ui::SH {
            for x in 0..calc_ui::SW {
                let px = bx + x;
                let py = by + y;
                if px < W && py < H {
                    fb[py * W + px] = surf[y * calc_ui::SW + x];
                }
            }
        }
        return;
    }
    // Titolo: "/" più la cartella corrente del terminale (es. "/DOCS"), fino a 16 caratteri.
    let mut title = [0u8; 17];
    title[0] = b'/';
    let cwd = d.term.cwd();
    let tn = upper_into(&mut title[1..], cwd);
    crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, &title[..1 + tn], t0);
    // Prompt: "$ " e la voce del menu scelta con su/giù. A la esegue.
    let mut prompt = [0u8; terminal::INPUT_MAX + terminal::LABEL_MAX + 4];
    prompt[0] = b'$';
    prompt[1] = b' ';
    let n = if d.term.typing {
        // Digitazione: testo scritto e carattere scelto (lo spazio si vede come _).
        let inp = d.term.input();
        let mut n = upper_into(&mut prompt[2..], inp);
        let c = terminal::ALPHA[d.term.pick];
        prompt[2 + n] = if c == b' ' { b'_' } else { c };
        n += 1;
        n
    } else {
        let mut lab = [0u8; terminal::LABEL_MAX];
        let ln = d.term.label(fs, d.term.sel, &mut lab);
        upper_into(&mut prompt[2..], &lab[..ln])
    };
    crate::font::draw(fb, W, H, WIN_X + 4, by + 2, &prompt[..2 + n], t0);
    // Output: sotto il prompt, una riga per voce.
    for k in 0..d.term.lines() {
        let mut out = [0u8; terminal::LINE_MAX];
        let n = upper_into(&mut out, d.term.line(k));
        crate::font::draw(fb, W, H, WIN_X + 4, by + 10 + k * 7, &out[..n], t0);
    }
    put(fb, WIN_X + 1, WIN_Y + 1, t3);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(d: &Desktop) -> [u16; W * H] {
        let mut surf = [0u16; calc_ui::SW * calc_ui::SH];
        d.draw_calc(&mut surf);
        let mut fb = [0u16; W * H];
        let media = [0u16; media_ui::W * media_ui::H];
        paint(&mut fb, d, &surf, &media, &Fs::new());
        fb
    }

    #[test]
    fn navigation_wraps_and_terminal_opens_and_closes() {
        let mut d = Desktop::new();
        assert!(d.step(KEY_RIGHT, &Fs::new()));
        assert_eq!(d.sel(), 1);
        assert!(!d.step(KEY_RIGHT, &Fs::new())); // tasto ancora premuto: nessun passo in più
        d.step(0, &Fs::new());
        assert!(d.step(KEY_RIGHT, &Fs::new()));
        assert_eq!(d.sel(), 2);
        d.step(0, &Fs::new());
        assert!(d.step(KEY_A, &Fs::new())); // sel 2 = CALC: apre la calcolatrice
        assert_eq!(d.win, Some(App::Calc));
        d.step(0, &Fs::new());
        assert!(d.step(KEY_B, &Fs::new()));
        assert!(!d.win_open());
        d.step(0, &Fs::new());
        d.step(KEY_LEFT, &Fs::new());
        assert_eq!(d.sel(), TERM_ICON);
        d.step(0, &Fs::new());
        assert!(d.step(KEY_A, &Fs::new()));
        assert!(d.win_open());
        d.step(0, &Fs::new());
        assert!(d.step(KEY_B, &Fs::new()));
        assert!(!d.win_open());
        d.step(0, &Fs::new());
        d.step(KEY_UP, &Fs::new());
        assert_eq!(d.sel(), 0);
        d.step(0, &Fs::new());
        d.step(KEY_LEFT, &Fs::new());
        assert_eq!(d.sel(), ICONS - 1);
    }

    #[test]
    fn calculator_takes_keys_while_open() {
        let mut d = Desktop::new();
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        d.step(KEY_A, &Fs::new()); // apre CALC
        d.step(0, &Fs::new());
        // Croce e A vanno alla calcolatrice: A preme il tasto 7 (cursore in 0).
        assert!(d.step(KEY_A, &Fs::new()));
        assert_eq!(d.calc().calc().shown(), 700);
        assert_eq!(d.sel(), CALC_ICON); // la selezione delle icone non si muove
        d.step(0, &Fs::new());
        assert!(d.step(KEY_RIGHT, &Fs::new()));
        assert_eq!(d.calc().cursor(), 1);
    }

    #[test]
    fn launcher_table_maps_icons_to_apps() {
        assert_eq!(launchable(0), Some(App::Files));
        assert_eq!(launchable(TERM_ICON), Some(App::Term));
        assert_eq!(launchable(CALC_ICON), Some(App::Calc));
        assert_eq!(launchable(3), Some(App::Notes));
        assert_eq!(launchable(ICONS + 5), None);
    }

    #[test]
    fn a_on_files_requests_fullscreen_run_not_window() {
        let mut d = Desktop::new();
        d.step(KEY_A, &Fs::new()); // icona 0 = FILES
        assert!(d.run_requested());
        assert!(!d.win_open());
        assert_eq!(d.take_run(), Some(App::Files));
        assert!(!d.run_requested());
    }

    #[test]
    fn notes_icon_requests_editor_run() {
        let mut d = Desktop::new();
        for _ in 0..3 {
            d.step(KEY_RIGHT, &Fs::new());
            d.step(0, &Fs::new());
        }
        assert_eq!(d.sel(), 3); // NOTES
        d.step(KEY_A, &Fs::new());
        assert_eq!(d.take_run(), Some(App::Notes));
    }

    #[test]
    fn bar_is_on_top_and_background_below() {
        let fb = screen(&Desktop::new());
        assert_eq!(fb[0], BAR);
        assert_eq!(fb[(BAR_H - 1) * W + 100], BAR);
        assert_eq!(fb[(BAR_H + 2) * W + 1], BG);
    }

    #[test]
    fn icons_sit_on_the_grid() {
        let fb = screen(&Desktop::new());
        let (x, y) = icon_pos(0);
        assert_eq!(fb[(y + 1) * W + x + 1], ICON_HI);
        let (x, y) = icon_pos(1);
        assert_eq!(fb[(y + 1) * W + x + 1], ICON);
        assert_eq!(icon_pos(4).0, icon_pos(0).0); // quinta icona torna a colonna 0
    }

    #[test]
    fn window_has_title_body_and_close_button() {
        let mut d = Desktop::new();
        d.step(KEY_RIGHT, &Fs::new()); // icona del terminale
        d.step(0, &Fs::new());
        d.step(KEY_A, &Fs::new());
        let fb = screen(&d);
        assert_eq!(fb[(WIN_Y + 2) * W + WIN_X + 3], WIN_TITLE);
        assert_eq!(fb[(WIN_Y + 3) * W + WIN_X + WIN_W - 5], CLOSE);
        assert_eq!(fb[(WIN_Y + TITLE_H + 3) * W + WIN_X + 3], WIN_BODY);
        assert_eq!(fb[WIN_Y * W + WIN_X], WIN_BORDER);
    }

    #[test]
    fn calculator_surface_is_inside_window_body() {
        let mut d = Desktop::new();
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        d.step(KEY_A, &Fs::new());
        let mut surf = [0u16; calc_ui::SW * calc_ui::SH];
        d.draw_calc(&mut surf);
        let fb = screen(&d);
        assert_eq!(fb[(WIN_Y + TITLE_H + 1) * W + WIN_X + 1], surf[0]);
        assert!(WIN_X + 1 + calc_ui::SW <= WIN_X + WIN_W);
        assert!(WIN_Y + TITLE_H + 1 + calc_ui::SH <= WIN_Y + WIN_H);
    }

    #[test]
    fn only_the_four_tints_are_used() {
        let mut d = Desktop::new();
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        d.step(KEY_A, &Fs::new());
        let fb = screen(&d);
        for px in fb.iter() {
            assert!([T0, T1, T2, T3].contains(px), "colore fuori tavolozza: {px:#06x}");
        }
    }

    #[test]
    fn window_fits_on_screen() {
        assert!(WIN_X + WIN_W <= W);
        assert!(WIN_Y + WIN_H <= H);
    }
}

#[cfg(test)]
mod tests_apps {
    use super::*;
    use crate::apps::settings::PALETTES;

    fn screen(d: &Desktop) -> Vec<u16> {
        let mut surf = [0u16; calc_ui::SW * calc_ui::SH];
        d.draw_calc(&mut surf);
        let mut fb = vec![0u16; W * H];
        let media = [0u16; media_ui::W * media_ui::H];
        paint(&mut fb, d, &surf, &media, &Fs::new());
        fb
    }

    fn open(d: &mut Desktop, icona: usize, app: App) {
        while d.sel() != icona {
            d.step(KEY_RIGHT, &Fs::new());
            d.step(0, &Fs::new());
        }
        d.step(KEY_A, &Fs::new());
        d.step(0, &Fs::new());
        assert_eq!(d.win, Some(app));
    }

    #[test]
    fn tema_cambia_palette_sullo_schermo() {
        let mut d = Desktop::new();
        assert_eq!(screen(&d)[(H - 1) * W], PALETTES[0][0]);
        open(&mut d, 5, App::Settings);
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        assert_eq!(d.settings.theme(), 1);
        let fb = screen(&d);
        assert_eq!(fb[(H - 1) * W], PALETTES[1][0]); // sfondo
        assert_eq!(fb[(WIN_Y + WIN_H - 3) * W + WIN_X + WIN_W - 3], PALETTES[1][1]); // corpo finestra
    }

    #[test]
    fn orologio_avanza_e_si_regola() {
        let mut d = Desktop::new();
        d.clock = Clock::new(9, 5);
        let mut cambiato = false;
        for _ in 0..60 * 60 {
            cambiato |= d.tick();
        }
        assert!(cambiato);
        assert_eq!(d.clock.testo(), *b"09:06");
        open(&mut d, 4, App::Clock);
        assert!(d.step(KEY_RIGHT, &Fs::new())); // +1 ora: il desktop si ridisegna
        d.step(0, &Fs::new());
        assert_eq!(d.clock.testo(), *b"10:06");
    }

    /// Genera PPM del desktop (`OUT=cartella cargo`-free): `rustc --test host/test_root.rs && OUT=dir ./test --ignored screenshot`.
    #[test]
    #[ignore]
    fn screenshot_app_di_sistema() {
        let dir = std::env::var("OUT").unwrap_or(".".into());
        let salva = |nome: &str, d: &Desktop| {
            let fb = screen(d);
            let mut out = format!("P6\n{} {}\n255\n", W, H).into_bytes();
            for &c in &fb {
                let r = (c & 31) as u32 * 255 / 31;
                let g = ((c >> 5) & 31) as u32 * 255 / 31;
                let b = ((c >> 10) & 31) as u32 * 255 / 31;
                out.extend_from_slice(&[r as u8, g as u8, b as u8]);
            }
            std::fs::write(format!("{}/{}.ppm", dir, nome), out).unwrap();
        };
        let mut d = Desktop::new();
        salva("1_home", &d);
        d.clock = Clock::new(9, 5);
        open(&mut d, 4, App::Clock);
        salva("2_orologio", &d);
        d.step(KEY_B, &Fs::new());
        d.step(0, &Fs::new());
        open(&mut d, 5, App::Settings);
        d.step(KEY_RIGHT, &Fs::new());
        d.step(0, &Fs::new());
        salva("3_impostazioni", &d);
    }
}
