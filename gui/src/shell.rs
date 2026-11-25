//! Scrivania: icone, finestre, regole di input. Nessun disegno qui.
use crate::input::Key;
use crate::layout::{content, icon_cell, COLS};

/// Icone per pagina: 3 righe da 4. La quarta riga finirebbe con l'etichetta fuori dallo schermo.
pub const PER_PAGE: usize = 12;
use crate::wm::Wm;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Icon {
    pub label: &'static str,
    pub app: u8,
    pub title: &'static str,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    None,
    /// Un'app è stata aperta o portata in primo piano.
    Launch(u8),
    /// Una finestra è stata chiusa.
    Closed(u8),
    /// Tasto inoltrato all'app della finestra attiva.
    ToApp(Key),
    /// Tocco dentro il contenuto dell'app attiva (coordinate relative).
    Tap { app: u8, x: i32, y: i32 },
}

pub struct Shell {
    pub icons: &'static [Icon],
    pub sel: usize,
    pub wm: Wm,
    pub w: i32,
    pub h: i32,
    /// true: il fuoco è sul desktop; false: sulla finestra attiva.
    pub desk: bool,
}

impl Shell {
    pub const fn new(icons: &'static [Icon], w: i32, h: i32) -> Shell {
        Shell { icons, sel: 0, wm: Wm::new(), w, h, desk: true }
    }

    pub fn pages(&self) -> usize {
        self.icons.len().div_ceil(PER_PAGE).max(1)
    }

    pub fn page(&self) -> usize {
        self.sel / PER_PAGE
    }

    /// Icone visibili nella pagina corrente: indice assoluto della prima e numero.
    pub fn visible(&self) -> (usize, usize) {
        let start = self.page() * PER_PAGE;
        (start, (self.icons.len() - start).min(PER_PAGE))
    }

    /// Apre l'app `i` o la porta in primo piano: mai due copie.
    fn launch(&mut self, i: usize) -> Event {
        let ic = self.icons[i];
        if let Some(j) = self.wm.find_app(ic.app) {
            self.wm.raise(j);
            self.desk = false;
            return Event::Launch(ic.app);
        }
        if self.wm.open(ic.app, ic.title, ic.w, ic.h, self.w, self.h) {
            self.desk = false;
            Event::Launch(ic.app)
        } else {
            Event::None
        }
    }

    /// Tasto. Con una finestra aperta: B chiude, Select cambia finestra, il resto va all'app.
    /// Sul desktop: croce sceglie l'icona, A la apre.
    pub fn key(&mut self, k: Key) -> Event {
        if !self.desk && self.wm.top().is_some() {
            return match k {
                Key::B => match self.wm.close(self.wm.len() - 1) {
                    Some(app) => Event::Closed(app),
                    None => Event::None,
                },
                Key::Select | Key::R => {
                    self.wm.cycle();
                    Event::None
                }
                Key::Start => {
                    self.desk = true;
                    Event::None
                }
                _ => Event::ToApp(k),
            };
        }
        if matches!(k, Key::B | Key::Start) && self.wm.top().is_some() {
            self.desk = false;
            return Event::None;
        }
        if k == Key::Select && self.wm.top().is_some() && self.icons.len() <= PER_PAGE {
            self.desk = false;
            return Event::None;
        }
        let n = self.icons.len();
        let local = self.sel % PER_PAGE;
        match k {
            Key::Left if !local.is_multiple_of(COLS) => self.sel -= 1,
            Key::Right if local % COLS + 1 < COLS && self.sel + 1 < n => self.sel += 1,
            Key::Up if local >= COLS => self.sel -= COLS,
            Key::Down if local + COLS < PER_PAGE && self.sel + COLS < n => self.sel += COLS,
            Key::Select if n > PER_PAGE => {
                // Select sul desktop (nessuna finestra): pagina successiva.
                self.sel = ((self.page() + 1) % self.pages()) * PER_PAGE;
            }
            Key::A if n > 0 => return self.launch(self.sel),
            _ => {}
        }
        Event::None
    }

    /// Puntatore (x86): click su icona, pulsante chiudi, contenuto o finestra.
    pub fn click(&mut self, x: i32, y: i32) -> Event {
        if let Some(i) = self.wm.hit(x, y) {
            if self.wm.close_hit(i, x, y) {
                return match self.wm.close(i) {
                    Some(app) => Event::Closed(app),
                    None => Event::None,
                };
            }
            self.desk = false;
            let top = self.wm.len() - 1;
            let frame = self.wm.get(i).map(|w| w.frame);
            if i == top {
                if let (Some(w), Some(f)) = (self.wm.top(), frame) {
                    let c = content(f);
                    if c.contains(x, y) {
                        return Event::Tap { app: w.app, x: x - c.x, y: y - c.y };
                    }
                }
            } else {
                self.wm.raise(i);
            }
            return Event::None;
        }
        let (start, count) = self.visible();
        for k in 0..count {
            if icon_cell(k).contains(x, y) {
                self.sel = start + k;
                return self.launch(start + k);
            }
        }
        Event::None
    }
}
