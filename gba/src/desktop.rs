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
/// Icone 16x16 nella griglia della bibbia (§5), dal crate `haiku-gui`: 4 colonne, passo 48x40,
/// da (8, 24). Una pagina ha 3 righe (12 icone): la quarta riga porterebbe l'etichetta fuori schermo.
/// Select sul desktop cambia pagina.
pub const ICON_SIZE: usize = haiku_gui::layout::ICON as usize;
pub const ICON_COLS: usize = haiku_gui::layout::COLS;
pub const ICON_STEP_X: usize = haiku_gui::layout::PITCH_X as usize;
pub const ICON_STEP_Y: usize = haiku_gui::layout::PITCH_Y as usize;
pub const PER_PAGE: usize = 12;
/// Finestra del terminale: posizione e misure.
pub const WIN_X: usize = 124;
pub const WIN_Y: usize = 30;
pub const WIN_W: usize = 108;
pub const WIN_H: usize = 88;
pub const TITLE_H: usize = 10;
/// Numero di icone del lanciatore (una pagina: 5 colonne x 4 righe).
pub const ICONS: usize = 29;
/// Etichette sotto le icone. Il terminale digita questi nomi (vedi terminal::APP_CMDS).
pub const ICON_LABELS: [&[u8]; ICONS] = [
    b"FILES", b"TERM", b"CALC", b"NOTES", b"CLOCK", b"SETTINGS", b"CHESS", b"READ", b"REMIND", b"PASSWORDS", b"DICTIONARY", b"CALENDAR", b"MESSAGES", b"PHOTOS", b"VIEWER", b"CAMERA", b"MUSIC", b"MAPS", b"MEMO", b"UTIL",
    b"SOFTWARE", b"AIUTO", b"TOUR", b"SCANNER",
    b"TIMER", b"CHESSCLOCK", b"TALLY", b"LOREM", b"SHRED",
];
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
    /// Promemoria: lista a 4 voci.
    Reminders,
    /// Dieci app di sistema (logica in apps/*.rs, stato in apps/ui.rs). Tasti in `AppsUi::key`.
    Passwords,
    Dictionary,
    Calendar,
    Messages,
    Photos,
    PhotoView,
    Camera,
    Music,
    Maps,
    Memo,
    /// Utility di sistema: disco, memoria, stampante, nota, colore (sola lettura, A aggiunge un lavoro).
    Utility,
    /// Software (catalogo), Aiuto, Visita guidata, Acquisizione documenti (logica in apps/*.rs).
    Software,
    Help,
    Tour,
    Scanner,
    /// Timer per esercizi (A pausa, Select azzera).
    Timer,
    /// Orologio per scacchi (L e R premono i giocatori).
    ChessClock,
    /// Contatore (su/giù, Select azzera).
    Tally,
    /// Testo segnaposto (A cambia testo).
    Lorem,
    /// Distruggi file: sul buffer di prova (A distrugge).
    Shred,
}

/// Tabella del lanciatore: programma dietro a ogni icona.
pub const APPS: [App; ICONS] = [
    App::Files, App::Term, App::Calc, App::Notes, App::Clock, App::Settings, App::Chess, App::Reader, App::Reminders,
    App::Passwords, App::Dictionary, App::Calendar, App::Messages, App::Photos, App::PhotoView, App::Camera, App::Music, App::Maps, App::Memo,
    App::Utility, App::Software, App::Help, App::Tour, App::Scanner,
    App::Timer, App::ChessClock, App::Tally, App::Lorem, App::Shred,
];

/// Programma lanciabile dalla icona `i`, se c'è una finestra per lui.
pub fn launchable(i: usize) -> Option<App> {
    match APPS.get(i).copied().unwrap_or(App::None) {
        App::None => None,
        a => Some(a),
    }
}

use crate::apps::{aiuto::Aiuto, clock::Clock, scanner::{self, Evento, Scanner, Stato}, settings::Settings, software::{self, Software}, sysutil, tour::Tour, ui::AppsUi, view};
use crate::media_ui::{self, ChessView, Key, Reader};
use crate::calc_ui::{self, CalcUi};
use crate::reminders::Reminders;
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
    pub rem: Reminders,
    pub apps: AppsUi,
    sw: Software,
    aiuto: Aiuto,
    tour: Tour,
    scan: Scanner,
}

impl Desktop {
    pub fn new() -> Self {
        Self { sel: 0, win: None, prev: 0, calc: CalcUi::new(), term: Terminal::new(), run: None, clock: Clock::new(9, 0), settings: Settings::new(), chess: ChessView::new(), reader: Reader::new(READER_TEXT), rem: Reminders::new(), apps: AppsUi::new(), sw: Software::new(), aiuto: Aiuto::new(), tour: Tour::new(), scan: Scanner::new() }
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
        let prima = (self.clock.testo(), self.apps.timer.secondi(), self.apps.scacchi.secondi(0), self.apps.scacchi.secondi(1));
        self.clock.tick(1);
        self.apps.timer.tick(1);
        self.apps.scacchi.tick(1);
        prima != (self.clock.testo(), self.apps.timer.secondi(), self.apps.scacchi.secondi(0), self.apps.scacchi.secondi(1))
    }

    /// Due promemoria di prova, da mostrare al primo avvio (la lista parte vuota).
    pub fn demo_reminders(&mut self) {
        self.rem.add(b"TELEFONA ANNA");
        self.rem.add(b"LEGGI MAIL");
    }

    /// Stato della lista promemoria, per sapere se la finestra va ridisegnata.
    fn rem_digest(&self) -> (usize, usize, usize) {
        (self.rem.sel, self.rem.open(), self.rem.filled())
    }

    /// Tasti della lista promemoria. Fuori da `step` (inline(never)): dentro `step` la ROM non passava l'harness.
    #[inline(never)]
    fn rem_keys(&mut self, pressed: u16) {
        if pressed & KEY_UP != 0 {
            self.rem.step(-1);
        }
        if pressed & KEY_DOWN != 0 {
            self.rem.step(1);
        }
        if pressed & KEY_A != 0 {
            self.rem.toggle();
        }
        if pressed & KEY_SELECT != 0 {
            self.rem.remove();
        }
        if pressed & KEY_B != 0 {
            self.win = None;
        }
    }

    pub fn step(&mut self, keys: u16, fs: &Fs) -> bool {
        let pressed = keys & !self.prev;
        self.prev = keys;
        // Scacchi e lettore: il cambio di stato va segnalato, altrimenti la finestra non si ridisegna.
        let mut media_changed = false;
        let mut app_changed = false;
        let rb = self.rem_digest();
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
                if pressed != 0 {
                    media_changed = true;
                }
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
            Some(App::Software) => {
                if pressed & KEY_UP != 0 {
                    self.sw.su();
                }
                if pressed & KEY_DOWN != 0 {
                    self.sw.giu();
                }
                if pressed & KEY_A != 0 {
                    self.sw.commuta();
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Help) => {
                if pressed & KEY_UP != 0 {
                    self.aiuto.su();
                }
                if pressed & KEY_DOWN != 0 {
                    self.aiuto.giu();
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Tour) => {
                if pressed & KEY_A != 0 {
                    self.tour.avanti();
                }
                if pressed & KEY_LEFT != 0 {
                    self.tour.indietro();
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Scanner) => {
                if pressed & KEY_A != 0 {
                    let e = if self.scan.stato() == Stato::Spento { Evento::Accendi } else { Evento::Scansiona };
                    self.scan.evento(e);
                }
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Calc) => {
                self.calc.step(keys);
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Reminders) => self.rem_keys(pressed),
            Some(App::Passwords) => {
                app_changed = self.apps.key_password(pressed, fs);
                if pressed & KEY_B != 0 {
                    self.win = None;
                    self.apps.cassa.chiudi();
                }
            }
            Some(a @ (App::Dictionary | App::Calendar | App::Photos | App::PhotoView | App::Maps | App::Timer | App::ChessClock | App::Tally | App::Lorem | App::Shred)) => {
                app_changed = self.apps.key(a, pressed);
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            // Solo hardware: nessun tasto oltre a B, che chiude.
            Some(App::Camera | App::Music | App::Messages | App::Memo) => {
                if pressed & KEY_B != 0 {
                    self.win = None;
                }
            }
            Some(App::Utility) => {
                if pressed & KEY_A != 0 {
                    self.term.spool.submit();
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
                        // CHESS e READ dal terminale aprono la finestra corrispondente.
                        match self.term.take_open() {
                            Some(terminal::Open::Chess) => self.win = Some(App::Chess),
                            Some(terminal::Open::Reader) => self.win = Some(App::Reader),
                            None => {}
                        }
                        if let Some((buf, n)) = self.term.take_add() {
                            self.rem.add(&buf[..n]);
                        }
                        if let Some(i) = self.term.take_launch() {
                            match launchable(i) {
                                Some(App::Files) | Some(App::Notes) => self.run = launchable(i),
                                Some(app) => self.win = Some(app),
                                None => {}
                            }
                        }
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
                // Griglia della pagina: frecce non escono dalla pagina; Select passa alla pagina successiva.
                let local = self.sel % PER_PAGE;
                if pressed & KEY_RIGHT != 0 && local % ICON_COLS + 1 < ICON_COLS && self.sel + 1 < ICONS {
                    self.sel += 1;
                }
                if pressed & KEY_LEFT != 0 && local % ICON_COLS > 0 {
                    self.sel -= 1;
                }
                if pressed & KEY_DOWN != 0 && local + ICON_COLS < PER_PAGE && self.sel + ICON_COLS < ICONS {
                    self.sel += ICON_COLS;
                }
                if pressed & KEY_UP != 0 && local >= ICON_COLS {
                    self.sel -= ICON_COLS;
                }
                if pressed & KEY_SELECT != 0 && ICONS > PER_PAGE {
                    let pages = ICONS.div_ceil(PER_PAGE);
                    self.sel = ((self.sel / PER_PAGE + 1) % pages) * PER_PAGE;
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
        media_changed
            || app_changed
            || before
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
            || rb != self.rem_digest()
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

/// Posizione (x, y) dell'icona `i` nella sua pagina (cella del crate `haiku-gui`).
pub fn icon_pos(i: usize) -> (usize, usize) {
    let c = haiku_gui::layout::icon_cell(i % PER_PAGE);
    (c.x as usize, c.y as usize)
}

/// Prima icona della pagina che contiene `sel`.
pub fn page_start(sel: usize) -> usize {
    sel / PER_PAGE * PER_PAGE
}

/// Titolo della finestra delle dieci app di sistema (None per le altre).
fn titolo_app(app: App) -> Option<&'static [u8]> {
    let t: &'static [u8] = match app {
        App::Passwords => b"PASSWORDS",
        App::Dictionary => b"DICTIONARY",
        App::Calendar => b"CALENDAR",
        App::Messages => b"MESSAGES",
        App::Photos => b"PHOTOS",
        App::PhotoView => b"VIEWER",
        App::Camera => b"CAMERA",
        App::Music => b"MUSIC",
        App::Maps => b"MAPS",
        App::Memo => b"MEMO",
        App::Software => b"SOFTWARE",
        App::Help => b"AIUTO",
        App::Tour => b"TOUR",
        App::Scanner => b"SCANNER",
        App::Timer => b"TIMER",
        App::ChessClock => b"CHESSCLOCK",
        App::Tally => b"TALLY",
        App::Lorem => b"LOREM",
        App::Shred => b"SHRED",
        _ => return None,
    };
    Some(t)
}

/// Chiave del glifo condiviso (`haiku_gui::icons::cuts`) per l'etichetta dell'icona.
fn glifo(label: &[u8]) -> &'static str {
    match label {
        b"FILES" => "FILE",
        b"SETTINGS" => "SET",
        b"NOTES" => "NOTE",
        b"TERM" => "TERM",
        b"CALC" => "CALC",
        b"CLOCK" => "CLOCK",
        b"READ" => "READ",
        b"REMIND" => "REMIND",
        b"PASSWORDS" => "PASSWORDS",
        b"DICTIONARY" => "DICTIONARY",
        b"CALENDAR" => "CALENDAR",
        _ => "",
    }
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
    crate::font::draw(fb, W, H, W - 30, 3, &d.clock.testo(), t0);

    // Icone con etichetta. Riquadro chiaro = icona selezionata.
    let first = page_start(d.sel());
    for i in first..(first + PER_PAGE).min(ICONS) {
        let (x, y) = icon_pos(i);
        if i == d.sel() {
            // Selezionata: cella piena T1, segni in T0 (bibbia §8).
            rect(fb, x, y, ICON_SIZE, ICON_SIZE, t1);
        } else {
            let (bx, by, bw, bh) = haiku_gui::icons::BLOCK;
            rect(fb, x + bx as usize, y + by as usize, bw as usize, bh as usize, t2);
        }
        for &(cx, cy, cw, ch) in haiku_gui::icons::cuts(glifo(ICON_LABELS[i])) {
            rect(fb, x + cx as usize, y + cy as usize, cw as usize, ch as usize, t0);
        }
        crate::font::draw(fb, W, H, x, y + ICON_SIZE + 2, ICON_LABELS[i], T1);
    }
    let app = match d.win {
        Some(a) => a,
        None => return,
    };

    // Finestra: titolo, corpo con bordo di 1 px, pulsante chiudi 8x8.
    rect(fb, WIN_X, WIN_Y, WIN_W, WIN_H, t3);
    rect(fb, WIN_X + 1, WIN_Y + 1, WIN_W - 2, TITLE_H, t2);
    rect(fb, WIN_X + WIN_W - 10, WIN_Y + 1, 8, 8, t1);
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
    if app == App::Reminders {
        crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, b"REMIND", TEXT);
        for i in 0..crate::reminders::SLOTS {
            if let Some(it) = d.rem.item(i) {
                // Riga: * se selezionata, + fatta o - aperta, poi il testo.
                let mut row = [b' '; 3 + crate::reminders::TEXT_MAX];
                row[0] = if i == d.rem.sel { b'*' } else { b' ' };
                row[1] = if it.done { b'+' } else { b'-' };
                row[3..3 + it.len].copy_from_slice(&it.text[..it.len]);
                crate::font::draw(fb, W, H, WIN_X + 4, by + 2 + i * 10, &row[..3 + it.len], TEXT);
            }
        }
        return;
    }
    if app == App::Utility {
        crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, b"UTIL", TEXT);
        // Disco, memoria, coda di stampa e nota di riferimento (La 440 Hz), due righe per voce.
        let env = d.term.env(fs);
        let outs = [sysutil::disk(&env), sysutil::info(&env), d.term.spool.out(), sysutil::note(69)];
        let mut y = by + 2;
        for (a, b) in outs {
            crate::font::draw(fb, W, H, WIN_X + 4, y, a.as_bytes(), TEXT);
            y += 10;
            if !b.is_empty() {
                crate::font::draw(fb, W, H, WIN_X + 4, y, b.as_bytes(), TEXT);
                y += 10;
            }
        }
        return;
    }
    // Software, Aiuto, Visita guidata, Scanner: testo in poche righe.
    if matches!(app, App::Software | App::Help | App::Tour | App::Scanner) {
        if let Some(titolo) = titolo_app(app) {
            crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, titolo, t0);
        }
        let mut y = by + 2;
        match app {
            App::Software => {
                for i in 0..software::CATALOGO.len() {
                    let c = if i == d.sw.sel() { t0 } else { TEXT };
                    crate::font::draw(fb, W, H, bx + 2, y, d.sw.nome(i).as_bytes(), c);
                    if d.sw.installato(i) {
                        crate::font::draw(fb, W, H, bx + 70, y, b"OK", c);
                    }
                    y += 10;
                }
            }
            App::Help => {
                crate::font::draw(fb, W, H, bx + 2, y, d.aiuto.titolo().as_bytes(), t0);
                y += 12;
                for r in d.aiuto.righe() {
                    crate::font::draw(fb, W, H, bx + 2, y, r.as_bytes(), TEXT);
                    y += 10;
                }
            }
            App::Tour => {
                for r in d.tour.righe() {
                    crate::font::draw(fb, W, H, bx + 2, y, r.as_bytes(), TEXT);
                    y += 10;
                }
            }
            _ => {
                let righe: &[&str] = match d.scan.stato() {
                    Stato::Spento => &["PREMI A PER", "ACCENDERE"],
                    Stato::Pronto => &["PREMI A PER", "SCANSIONARE"],
                    Stato::Acquisito => &["DOCUMENTO OK"],
                    Stato::ScannerAssente => scanner::AVVISO,
                };
                for r in righe {
                    crate::font::draw(fb, W, H, bx + 2, y, r.as_bytes(), TEXT);
                    y += 10;
                }
            }
        }
        return;
    }
    // Dieci app di sistema: titolo, poi il corpo della loro schermata.
    if let Some(titolo) = titolo_app(app) {
        crate::font::draw(fb, W, H, WIN_X + 5, WIN_Y + 2, titolo, t0);
        let pal = d.settings.palette();
        let a = &d.apps;
        match app {
            App::Passwords => view::passwords(fb, W, H, bx, by, a, pal),
            App::Dictionary => view::dizionario(fb, W, H, bx, by, a, pal),
            App::Calendar => view::calendario(fb, W, H, bx, by, a, pal),
            App::Photos => view::foto(fb, W, H, bx, by, a, pal),
            App::PhotoView => view::visualizzatore(fb, W, H, bx, by, a, pal),
            App::Maps => view::mappa(fb, W, H, bx, by, a, pal),
            App::Timer => view::timer(fb, W, H, bx, by, a, pal),
            App::ChessClock => view::scacchi_orologio(fb, W, H, bx, by, a, pal),
            App::Tally => view::contatore(fb, W, H, bx, by, a, pal),
            App::Lorem => view::testo_segnaposto(fb, W, H, bx, by, a, pal),
            App::Shred => view::distruggi(fb, W, H, bx, by, a, pal),
            App::Camera => view::da_verificare(fb, W, H, bx, by, b"SENSORE FOTO", pal),
            App::Music => view::da_verificare(fb, W, H, bx, by, b"AUDIO", pal),
            App::Messages => view::da_verificare(fb, W, H, bx, by, b"LINK", pal),
            _ => view::da_verificare(fb, W, H, bx, by, b"MICROFONO", pal),
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
        d.step(KEY_DOWN, &Fs::new());
        assert_eq!(d.sel(), TERM_ICON + ICON_COLS);
        d.step(0, &Fs::new());
        d.step(KEY_UP, &Fs::new());
        assert_eq!(d.sel(), TERM_ICON);
        d.step(0, &Fs::new());
        d.step(KEY_LEFT, &Fs::new());
        assert_eq!(d.sel(), 0);
        d.step(0, &Fs::new());
        d.step(KEY_LEFT, &Fs::new());
        assert_eq!(d.sel(), 0); // ai bordi della pagina le frecce non escono
    }

    #[test]
    fn select_turns_the_page_of_icons() {
        let mut d = Desktop::new();
        assert!(d.step(KEY_SELECT, &Fs::new()));
        assert_eq!(d.sel(), PER_PAGE);
        d.step(0, &Fs::new());
        assert!(d.step(KEY_SELECT, &Fs::new()));
        assert_eq!(d.sel(), 2 * PER_PAGE);
        d.step(0, &Fs::new());
        assert!(d.step(KEY_SELECT, &Fs::new()));
        assert_eq!(d.sel(), 0); // tre pagine: torna alla prima
    }

    #[test]
    fn icone_delle_app_a_tempo_aprono_la_finestra() {
        let nuove = [App::Timer, App::ChessClock, App::Tally, App::Lorem, App::Shred];
        for (k, app) in nuove.into_iter().enumerate() {
            assert_eq!(launchable(24 + k), Some(app));
        }
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
        assert_eq!(icon_pos(ICON_COLS).0, icon_pos(0).0); // prima icona della riga dopo torna a colonna 0
    }

    #[test]
    fn dodici_icone_per_pagina_entrano_nello_schermo_senza_sovrapporsi() {
        for i in 0..PER_PAGE {
            let (x, y) = icon_pos(i);
            assert!(x + ICON_SIZE <= W, "icona {i} fuori a destra");
            // Etichetta sotto l'icona: 2 px di spazio e 5 di altezza del font.
            assert!(y + ICON_SIZE + 2 + 5 <= H, "icona {i} fuori in basso");
            assert!(crate::font::text_width(ICON_LABELS[i]) < ICON_STEP_X, "etichetta {i} tocca la colonna dopo");
        }
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
        // Va all'icona: Select cambia pagina, poi colonna e riga della griglia.
        while d.sel() / PER_PAGE != icona / PER_PAGE {
            d.step(KEY_SELECT, &Fs::new());
            d.step(0, &Fs::new());
        }
        while d.sel() != icona {
            let s = d.sel();
            let k = if s % ICON_COLS < icona % ICON_COLS {
                KEY_RIGHT
            } else if s % ICON_COLS > icona % ICON_COLS {
                KEY_LEFT
            } else if s < icona {
                KEY_DOWN
            } else {
                KEY_UP
            };
            d.step(k, &Fs::new());
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

    /// Le dieci app nuove, nell'ordine delle icone 9..19.
    const NUOVE: [App; 10] = [App::Passwords, App::Dictionary, App::Calendar, App::Messages, App::Photos, App::PhotoView, App::Camera, App::Music, App::Maps, App::Memo];

    /// Digita `parola` nel terminale aperto: SELECT, poi A per lettera, sinistra per eseguire.
    fn digita(d: &mut Desktop, parola: &[u8]) {
        d.step(KEY_SELECT, &Fs::new());
        d.step(0, &Fs::new());
        for &c in parola {
            d.term.pick = terminal::ALPHA.iter().position(|&a| a == c).unwrap();
            d.step(KEY_A, &Fs::new());
            d.step(0, &Fs::new());
        }
        d.step(KEY_LEFT, &Fs::new());
        d.step(0, &Fs::new());
    }

    #[test]
    fn icona_di_ogni_app_nuova_apre_la_finestra_e_b_chiude() {
        for (k, app) in NUOVE.into_iter().enumerate() {
            assert_eq!(launchable(9 + k), Some(app));
            let mut d = Desktop::new();
            open(&mut d, 9 + k, app);
            assert!(d.step(KEY_B, &Fs::new()));
            assert!(!d.win_open());
        }
    }

    #[test]
    fn terminale_apre_ogni_app_nuova_per_nome() {
        for (k, app) in NUOVE.into_iter().enumerate() {
            let mut d = Desktop::new();
            open(&mut d, TERM_ICON, App::Term);
            digita(&mut d, ICON_LABELS[9 + k]); // nome = etichetta dell'icona
            assert_eq!(d.win, Some(app));
        }
    }

    #[test]
    fn ogni_app_nuova_disegna_nel_corpo() {
        for (k, app) in NUOVE.into_iter().enumerate() {
            let mut d = Desktop::new();
            open(&mut d, 9 + k, app);
            let fb = screen(&d);
            let p = d.settings.palette();
            let testo = (WIN_Y + TITLE_H + 2..WIN_Y + WIN_H - 1).any(|y| (WIN_X + 1..WIN_X + WIN_W - 1).any(|x| fb[y * W + x] == p[0]));
            assert!(testo, "corpo vuoto per {:?}", app);
        }
    }

    #[test]
    fn password_mostra_il_segreto_con_a() {
        let mut d = Desktop::new();
        open(&mut d, 9, App::Passwords);
        d.apps.cassa.aperta = true;
        assert!(!d.apps.mostra);
        assert!(d.step(KEY_A, &Fs::new()));
        assert!(d.apps.mostra);
        assert_eq!(d.sel(), 9); // le croci non spostano l'icona mentre la finestra è aperta
    }

    #[test]
    fn calendario_cambia_mese_con_sinistra_e_destra() {
        let mut d = Desktop::new();
        open(&mut d, 11, App::Calendar);
        assert!(d.step(KEY_RIGHT, &Fs::new()));
        assert_eq!(d.apps.mese.mese, 12);
        d.step(0, &Fs::new());
        assert!(d.step(KEY_LEFT, &Fs::new()));
        d.step(0, &Fs::new());
        assert!(d.step(KEY_LEFT, &Fs::new()));
        assert_eq!(d.apps.mese.mese, 10);
    }
}
