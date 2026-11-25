//! Scrivania: icone, finestre, menu, pannello, regole di input. Nessun disegno qui.
use crate::apps::{Calc, Disk, FileView, Term, FILES, HOME, SCRIPT, TEXT_ROWS, THEMES};
use sysapps::browser::{parse_line, Browser, Line};
use sysapps::editor::Editor;
use sysapps::Store;
use crate::input::Key;
use crate::layout::*;
use crate::wm::Wm;

/// Icone per pagina: 3 righe da 4. La quarta riga finirebbe con l'etichetta fuori dallo schermo.
pub const PER_PAGE: usize = 12;
/// Lunghezza massima della ricerca nel menu.
pub const QMAX: usize = 12;
/// Voci mostrate nel menu.
pub const MENU_MAX: usize = 10;

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
    /// Orologio della barra "HH:MM".
    pub clock: [u8; 5],
    /// Menu applicazioni aperto.
    pub menu: bool,
    pub query: [u8; QMAX],
    pub qlen: usize,
    /// Voce evidenziata nel menu (indice tra le voci filtrate).
    pub msel: usize,
    /// Trascinamento in corso: app, offset del puntatore dall'angolo della finestra.
    drag: Option<(u8, i32, i32)>,
    pub calc: Calc,
    pub term: Term,
    pub files: FileView,
    pub browser: Browser<8>,
    /// Collegamento selezionato nella pagina.
    pub bsel: usize,
    pub editor: Editor<7, 27>,
    /// Prima riga visibile del browser, del lettore e uscita dell'ultimo script.
    pub bscroll: usize,
    pub rscroll: usize,
    pub eout: Term,
    /// Tema di colore (0..THEMES): usato dal disegno x86 e scelto da IMPOST o F2.
    pub theme: u8,
}

impl Shell {
    pub const fn new(icons: &'static [Icon], w: i32, h: i32) -> Shell {
        Shell {
            icons,
            sel: 0,
            wm: Wm::new(),
            w,
            h,
            desk: true,
            clock: *b"00:00",
            menu: false,
            query: [0; QMAX],
            qlen: 0,
            msel: 0,
            drag: None,
            calc: Calc::new(),
            term: Term::new(),
            files: FileView::new(),
            browser: Browser::new(),
            bsel: 0,
            editor: Editor::new(),
            bscroll: 0,
            rscroll: 0,
            eout: Term::new(),
            theme: 0,
        }
    }

    pub fn next_theme(&mut self) {
        self.theme = (self.theme + 1) % THEMES;
    }

    pub fn set_clock(&mut self, h: u8, m: u8) {
        self.clock = [b'0' + h / 10, b'0' + h % 10, b':', b'0' + m / 10, b'0' + m % 10];
    }

    pub fn query(&self) -> &str {
        core::str::from_utf8(&self.query[..self.qlen]).unwrap_or("")
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

    /// Voci del menu: icone la cui etichetta o titolo contiene la ricerca (maiuscole).
    pub fn match_count(&self) -> usize {
        let q = self.query();
        self.icons.iter().filter(|ic| ic.title.contains(q) || ic.label.contains(q)).count()
    }

    /// k-esima icona tra quelle che corrispondono alla ricerca.
    pub fn nth_match(&self, k: usize) -> Option<usize> {
        let q = self.query();
        self.icons.iter().enumerate().filter(|(_, ic)| ic.title.contains(q) || ic.label.contains(q)).nth(k).map(|(i, _)| i)
    }

    pub fn toggle_menu(&mut self) {
        self.menu = !self.menu;
        self.qlen = 0;
        self.msel = 0;
    }

    /// Caratteri: nel menu e nelle app della finestra attiva; Invio apre l'icona sul desktop.
    /// Esc torna al desktop.
    pub fn text(&mut self, c: u8) -> Event {
        if self.menu {
            return self.menu_text(c);
        }
        if self.desk || self.wm.top().is_none() {
            return if c == 0x0D && !self.icons.is_empty() { self.launch(self.sel) } else { Event::None };
        }
        if c == 0x1B {
            self.desk = true;
        } else if let Some(w) = self.wm.top() {
            self.app_text(w.app, c);
        }
        Event::None
    }

    fn menu_text(&mut self, c: u8) -> Event {
        match c {
            0x1B => self.menu = false,
            0x0D => {
                if let Some(i) = self.nth_match(self.msel) {
                    self.menu = false;
                    return self.launch(i);
                }
            }
            0x08 => {
                self.qlen = self.qlen.saturating_sub(1);
                self.msel = 0;
            }
            b' ' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' if self.qlen < QMAX => {
                self.query[self.qlen] = c.to_ascii_uppercase();
                self.qlen += 1;
                self.msel = 0;
            }
            _ => {}
        }
        Event::None
    }

    /// Stato iniziale di un'app appena aperta.
    fn init_app(&mut self, app: u8) {
        match app {
            4 => {
                let _ = self.browser.open(&Disk, HOME);
                self.bsel = 0;
                self.bscroll = 0;
            }
            8 => self.rscroll = 0,
            5 => {
                if let Some(text) = Disk.file(SCRIPT) {
                    let _ = self.editor.load(text);
                }
            }
            _ => {}
        }
    }

    fn app_text(&mut self, app: u8, c: u8) {
        match app {
            1 if c == 0x0D => self.files.open = !self.files.open,
            2 if c == 0x0D => self.term.enter(),
            2 => self.term.type_char(c),
            3 => self.calc.key(c),
            6 if c == 0x0D => self.next_theme(),
            4 => match c {
                0x0D => {
                    if self.browser.follow(&Disk, self.bsel).is_ok() {
                        self.bsel = 0;
                        self.bscroll = 0;
                    }
                }
                0x08 => {
                    if self.browser.back().is_some() {
                        self.bsel = 0;
                        self.bscroll = 0;
                    }
                }
                _ => {}
            },
            5 => match c {
                0x0D => {
                    let _ = self.editor.newline();
                }
                0x08 => {
                    let _ = self.editor.backspace();
                }
                0x05 => {
                    // F5: esegue il buffer; l'uscita va nel pannello sotto il testo.
                    self.eout = Term::new();
                    let eout = &mut self.eout;
                    let mut out = |s: &str| eout.add(s.as_bytes());
                    if self.editor.run(&Disk, &mut out).is_err() {
                        self.eout.add(b"ERRORE SCRIPT");
                    }
                }
                b' '..=b'~' => {
                    let _ = self.editor.insert(c);
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Frecce nell'app FILE: scelgono il file.
    fn app_key(&mut self, app: u8, k: Key) {
        match (app, k) {
            (1, _) if self.files.open => {}
            (1, Key::Up) if self.files.sel > 0 => self.files.sel -= 1,
            (1, Key::Down) if self.files.sel + 1 < FILES.len() => self.files.sel += 1,
            (4, Key::Up) if self.bsel > 0 => {
                self.bsel -= 1;
                self.follow_sel();
            }
            (4, Key::Down) if self.bsel + 1 < self.link_count() => {
                self.bsel += 1;
                self.follow_sel();
            }
            (8, Key::Up) if self.rscroll > 0 => self.rscroll -= 1,
            (8, Key::Down) if self.rscroll + 1 < self.file_lines() => self.rscroll += 1,
            (5, Key::Up) => self.editor.up(),
            (5, Key::Down) => self.editor.down(),
            (5, Key::Left) => self.editor.left(),
            (5, Key::Right) => self.editor.right(),
            _ => {}
        }
    }

    /// Riporta in vista il collegamento selezionato.
    fn follow_sel(&mut self) {
        let Some(body) = self.browser.current() else { return };
        let line = body.lines().enumerate().filter(|(_, l)| matches!(parse_line(l), Line::Link { .. })).nth(self.bsel).map(|(i, _)| i);
        if let Some(li) = line {
            if li < self.bscroll {
                self.bscroll = li;
            } else if li >= self.bscroll + TEXT_ROWS {
                self.bscroll = li + 1 - TEXT_ROWS;
            }
        }
    }

    /// Righe del file scelto in FILE (per il lettore).
    pub fn file_lines(&self) -> usize {
        FILES[self.files.sel].1.lines().count()
    }

    /// Collegamenti della pagina corrente del browser.
    pub fn link_count(&self) -> usize {
        self.browser.current().map_or(0, |b| b.lines().filter(|l| matches!(parse_line(l), Line::Link { .. })).count())
    }

    /// Apre l'app `i` o la porta in primo piano: mai due copie.
    fn launch(&mut self, i: usize) -> Event {
        let ic = self.icons[i];
        if let Some(j) = self.wm.find_app(ic.app) {
            self.wm.restore(j);
            self.desk = false;
            return Event::Launch(ic.app);
        }
        if self.wm.open(ic.app, ic.title, ic.w, ic.h, self.w, self.h) {
            self.desk = false;
            self.init_app(ic.app);
            Event::Launch(ic.app)
        } else {
            Event::None
        }
    }

    /// Tasto. Con il menu aperto: frecce scelgono, Enter apre, il resto va alla ricerca.
    /// Con una finestra aperta: B chiude, Select cambia finestra, il resto va all'app.
    /// Sul desktop: croce sceglie l'icona, A la apre.
    pub fn key(&mut self, k: Key) -> Event {
        if self.menu {
            let n = self.match_count();
            return match k {
                Key::Up if self.msel > 0 => {
                    self.msel -= 1;
                    Event::None
                }
                Key::Down if self.msel + 1 < n => {
                    self.msel += 1;
                    Event::None
                }
                Key::Start => match self.nth_match(self.msel) {
                    Some(i) => {
                        self.menu = false;
                        self.launch(i)
                    }
                    None => Event::None,
                },
                _ => Event::None, // A e B sono anche lettere della ricerca
            };
        }
        if !self.desk && self.wm.top().is_some() {
            if let Some(w) = self.wm.top() {
                self.app_key(w.app, k);
            }
            return match k {
                Key::B => match self.wm.active() {
                    Some(i) => match self.wm.close(i) {
                        Some(app) => Event::Closed(app),
                        None => Event::None,
                    },
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

    /// Pulsante della finestra `i` nella barra: entra prima dell'orologio.
    fn task_fits(&self, i: usize) -> bool {
        task_button(i).x + TASK_W <= clock_x(self.w) - 2
    }

    /// Puntatore premuto (x86). Barra: menu e pulsanti finestra. Menu: voce. Finestre: pulsanti,
    /// trascinamento dal titolo, contenuto. Desktop: icone.
    pub fn press(&mut self, x: i32, y: i32) -> Event {
        if y < BAR_H {
            if menu_button().contains(x, y) {
                self.toggle_menu();
                return Event::None;
            }
            for i in 0..self.wm.len() {
                if self.task_fits(i) && task_button(i).contains(x, y) {
                    let Some(w) = self.wm.get(i) else { break };
                    if w.min {
                        self.wm.restore(i);
                    } else if self.wm.active() == Some(i) {
                        self.wm.minimize(i);
                    } else {
                        self.wm.raise(i);
                    }
                    self.desk = self.wm.top().is_none();
                    break;
                }
            }
            return Event::None;
        }
        if self.menu {
            let rows = self.match_count().min(MENU_MAX);
            let r = menu_rect(rows);
            if r.contains(x, y) && y >= BAR_H + MENU_ROW {
                let k = ((y - BAR_H) / MENU_ROW - 1) as usize;
                if let Some(i) = self.nth_match(k) {
                    self.menu = false;
                    return self.launch(i);
                }
            }
            self.menu = false; // click fuori dal menu: lo chiude
            return Event::None;
        }
        if let Some(i) = self.wm.hit(x, y) {
            let Some(w) = self.wm.get(i) else { return Event::None };
            let f = w.frame;
            self.desk = false;
            if self.wm.close_hit(i, x, y) {
                return match self.wm.close(i) {
                    Some(app) => Event::Closed(app),
                    None => Event::None,
                };
            }
            if self.wm.max_hit(i, x, y) {
                self.wm.toggle_max(i, self.w, self.h);
                self.wm.raise(i);
                return Event::None;
            }
            if self.wm.min_hit(i, x, y) {
                self.wm.minimize(i);
                self.desk = self.wm.top().is_none();
                return Event::None;
            }
            if title_bar(f).contains(x, y) {
                self.wm.raise(i);
                self.drag = Some((w.app, x - f.x, y - f.y));
                return Event::None;
            }
            if self.wm.active() != Some(i) {
                self.wm.raise(i);
                return Event::None;
            }
            let c = content(f);
            if c.contains(x, y) {
                return Event::Tap { app: w.app, x: x - c.x, y: y - c.y };
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

    /// Puntatore trascinato con il tasto premuto: sposta la finestra afferrata.
    pub fn drag(&mut self, x: i32, y: i32) {
        if let Some((app, dx, dy)) = self.drag {
            if let Some(i) = self.wm.find_app(app) {
                self.wm.move_win(i, x - dx, y - dy, self.w, self.h);
            }
        }
    }

    /// Puntatore rilasciato.
    pub fn release(&mut self) {
        self.drag = None;
    }

    /// Clic completo (premi e rilascia).
    pub fn click(&mut self, x: i32, y: i32) -> Event {
        let e = self.press(x, y);
        self.release();
        e
    }
}
