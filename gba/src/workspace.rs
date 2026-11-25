//! Spazio di lavoro minimo: navigazione delle directory, apertura e creazione di file.
//! Comandi dal keypad: su/giù scelgono, A apre (directory o file), B torna su, Start crea un file.
#![allow(dead_code)]
use crate::fs::{parent_of, Fs, FILES, NAME_MAX};

pub const KEY_A: u16 = 1 << 0;
pub const KEY_B: u16 = 1 << 1;
pub const KEY_SELECT: u16 = 1 << 2;
pub const KEY_START: u16 = 1 << 3;
pub const KEY_UP: u16 = 1 << 6;
pub const KEY_DOWN: u16 = 1 << 7;
pub const KEY_L: u16 = 1 << 9;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    None,
    Dir,
    File,
    Create,
    /// Select+B: chiede conferma prima di formattare.
    FormatAsk,
    /// Formattazione dello spazio di sistema, confermata con A. Non tocca lo spazio utente.
    FormatSys,
    /// B nella radice: torna al desktop.
    Back,
}

pub struct Workspace {
    cwd: [u8; NAME_MAX],
    cwd_len: usize,
    sel: usize,
    open: Option<usize>,
    prev: u16,
    /// Da ridisegnare.
    pub dirty: bool,
    /// Schermata di conferma della formattazione aperta.
    confirm: bool,
}

impl Workspace {
    pub const fn new() -> Self {
        Self { cwd: [0; NAME_MAX], cwd_len: 0, sel: 0, open: None, prev: 0, dirty: true, confirm: false }
    }

    pub fn cwd(&self) -> &[u8] {
        &self.cwd[..self.cwd_len]
    }

    pub fn sel(&self) -> usize {
        self.sel
    }

    pub fn open_id(&self) -> Option<usize> {
        self.open
    }

    pub fn confirming(&self) -> bool {
        self.confirm
    }

    /// Spazio di lavoro che parte da `path` (la cartella del terminale). Se non è una directory, parte dalla radice.
    pub fn new_at(fs: &Fs, path: &[u8]) -> Self {
        let mut ws = Self::new();
        let ok = path.is_empty() || fs.find_name(path).map_or(false, |id| fs.is_dir(id));
        if ok && path.len() <= NAME_MAX {
            ws.set_cwd(path);
        }
        ws
    }

    fn set_cwd(&mut self, path: &[u8]) {
        self.cwd[..path.len()].copy_from_slice(path);
        self.cwd_len = path.len();
        self.sel = 0;
        self.open = None;
        self.dirty = true;
    }

    /// Un passo di input. `keys` = tasti premuti in questo frame (1 = premuto).
    pub fn step(&mut self, fs: &Fs, keys: u16) -> Action {
        let pressed = keys & !self.prev;
        self.prev = keys;
        if self.confirm {
            // Con la conferma aperta: A formatta, B annulla, il resto è ignorato.
            if pressed & KEY_A != 0 {
                self.confirm = false;
                self.dirty = true;
                return Action::FormatSys;
            }
            if pressed & KEY_B != 0 {
                self.confirm = false;
                self.dirty = true;
            }
            return Action::None;
        }
        let mut ids = [0usize; FILES];
        let n = fs.list(self.cwd(), &mut ids);
        if n > 0 && self.sel >= n {
            self.sel = 0;
        }
        if n > 0 && pressed & KEY_DOWN != 0 {
            self.sel = (self.sel + 1) % n;
            self.dirty = true;
        }
        if n > 0 && pressed & KEY_UP != 0 {
            self.sel = (self.sel + n - 1) % n;
            self.dirty = true;
        }
        if n > 0 && pressed & KEY_A != 0 {
            let id = ids[self.sel];
            if fs.is_dir(id) {
                let mut path = [0u8; NAME_MAX];
                let (name, _) = fs.info(id).unwrap_or((&[], 0));
                path[..name.len()].copy_from_slice(name);
                let len = name.len();
                self.set_cwd(&path[..len]);
                return Action::Dir;
            }
            self.open = Some(id);
            self.dirty = true;
            return Action::File;
        }
        let combo = KEY_SELECT | KEY_B;
        if keys & combo == combo && pressed & combo != 0 {
            self.confirm = true;
            self.dirty = true;
            return Action::FormatAsk;
        }
        if pressed & KEY_B != 0 {
            if self.cwd_len == 0 {
                return Action::Back;
            }
            let mut path = [0u8; NAME_MAX];
            let parent = parent_of(self.cwd()).unwrap_or(&[]);
            path[..parent.len()].copy_from_slice(parent);
            let len = parent.len();
            self.set_cwd(&path[..len]);
            return Action::Dir;
        }
        if pressed & KEY_START != 0 {
            return Action::Create;
        }
        Action::None
    }
}

/// Nome di un nuovo file dentro `cwd`: "cwd/f<n>". Lunghezza 0 se non entra.
pub fn child_name(cwd: &[u8], n: u8) -> ([u8; NAME_MAX], usize) {
    let mut out = [0u8; NAME_MAX];
    let prefix = if cwd.is_empty() { 0 } else { cwd.len() + 1 };
    if prefix + 2 > NAME_MAX {
        return (out, 0);
    }
    if !cwd.is_empty() {
        out[..cwd.len()].copy_from_slice(cwd);
        out[cwd.len()] = b'/';
    }
    out[prefix] = b'f';
    out[prefix + 1] = b'0' + (n % 10);
    (out, prefix + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Fs {
        let mut fs = Fs::new();
        fs.mkdir(b"docs", 0).unwrap();
        fs.create(b"docs/a", 0).unwrap();
        fs.create(b"b", 0).unwrap();
        fs
    }

    #[test]
    fn down_then_open_file() {
        let fs = tree();
        let mut ws = Workspace::new();
        assert_eq!(ws.step(&fs, KEY_DOWN), Action::None);
        assert_eq!(ws.sel(), 1);
        assert_eq!(ws.step(&fs, 0), Action::None); // rilascio
        assert_eq!(ws.step(&fs, KEY_A), Action::File);
        assert_eq!(ws.open_id(), fs.find_name(b"b"));
    }

    #[test]
    fn b_at_root_goes_back_to_desktop() {
        let fs = tree();
        let mut ws = Workspace::new();
        assert_eq!(tap(&mut ws, &fs, KEY_B), Action::Back);
        assert_eq!(tap(&mut ws, &fs, KEY_A), Action::Dir); // sel 0 = docs
        assert_eq!(tap(&mut ws, &fs, KEY_B), Action::Dir); // dentro docs: sale alla radice
        assert_eq!(tap(&mut ws, &fs, KEY_B), Action::Back);
    }

    #[test]
    fn new_at_starts_in_given_dir_or_root() {
        let fs = tree();
        assert_eq!(Workspace::new_at(&fs, b"docs").cwd(), b"docs");
        assert_eq!(Workspace::new_at(&fs, b"nope").cwd(), b"");
        assert_eq!(Workspace::new_at(&fs, b"").cwd(), b"");
    }

    #[test]
    fn enter_dir_and_back() {
        let fs = tree();
        let mut ws = Workspace::new();
        assert_eq!(ws.step(&fs, KEY_A), Action::Dir); // sel 0 = docs
        assert_eq!(ws.cwd(), b"docs");
        assert_eq!(ws.step(&fs, 0), Action::None);
        assert_eq!(ws.step(&fs, KEY_B), Action::Dir);
        assert_eq!(ws.cwd(), b"");
    }

    #[test]
    fn hold_does_not_repeat() {
        let fs = tree();
        let mut ws = Workspace::new();
        ws.step(&fs, KEY_DOWN);
        ws.step(&fs, KEY_DOWN);
        ws.step(&fs, KEY_DOWN);
        assert_eq!(ws.sel(), 1);
    }

    #[test]
    fn select_and_b_together_formats_system() {
        let fs = tree();
        let mut ws = Workspace::new();
        assert_eq!(ws.step(&fs, KEY_SELECT), Action::None);
        assert_eq!(ws.step(&fs, KEY_SELECT | KEY_B), Action::FormatAsk);
        assert!(ws.confirming());
        // Rilascio, poi B annulla: nessuna formattazione, nessuna risalita.
        assert_eq!(ws.step(&fs, 0), Action::None);
        assert_eq!(ws.step(&fs, KEY_B), Action::None);
        assert!(!ws.confirming());
        // Solo B da solo, nella radice: torna al desktop.
        assert_eq!(ws.step(&fs, 0), Action::None);
        assert_eq!(ws.step(&fs, KEY_B), Action::Back);
    }

    #[test]
    fn format_needs_confirm_with_a() {
        let fs = tree();
        let mut ws = Workspace::new();
        ws.step(&fs, KEY_SELECT | KEY_B);
        ws.step(&fs, 0);
        // Navigazione ignorata durante la conferma.
        assert_eq!(ws.step(&fs, KEY_DOWN), Action::None);
        assert_eq!(ws.sel(), 0);
        ws.step(&fs, 0);
        assert_eq!(ws.step(&fs, KEY_A), Action::FormatSys);
        assert!(!ws.confirming());
    }

    /// Un tocco: tasti premuti per un frame, poi rilascio.
    fn tap(ws: &mut Workspace, fs: &Fs, keys: u16) -> Action {
        let a = ws.step(fs, keys);
        ws.step(fs, 0);
        a
    }

    #[test]
    fn regression_all_commands_on_test_fs() {
        let mut fs = tree();
        let mut ws = Workspace::new();
        // Creazione: Start chiede un file, la root riceve "f0" dopo il passo in main.
        assert_eq!(tap(&mut ws, &fs, KEY_START), Action::Create);
        let (nm, n) = child_name(ws.cwd(), 0);
        assert_eq!(&nm[..n], b"f0");
        fs.create(&nm[..n], 0).unwrap();
        // Navigazione: giù, su, su con limite (wrap), apri file.
        assert_eq!(tap(&mut ws, &fs, KEY_DOWN), Action::None);
        assert_eq!(tap(&mut ws, &fs, KEY_UP), Action::None);
        assert_eq!(ws.sel(), 0);
        assert_eq!(tap(&mut ws, &fs, KEY_UP), Action::None);
        assert_eq!(ws.sel(), 2); // tre voci: docs, b, f0
        assert_eq!(tap(&mut ws, &fs, KEY_DOWN), Action::None);
        assert_eq!(ws.sel(), 0);
        assert_eq!(tap(&mut ws, &fs, KEY_DOWN), Action::None);
        assert_eq!(tap(&mut ws, &fs, KEY_A), Action::File);
        assert_eq!(ws.open_id(), fs.find_name(b"b"));
        // Directory: apri docs, poi B risale alla radice.
        assert_eq!(tap(&mut ws, &fs, KEY_UP), Action::None);
        assert_eq!(tap(&mut ws, &fs, KEY_A), Action::Dir);
        assert_eq!(ws.cwd(), b"docs");
        assert_eq!(tap(&mut ws, &fs, KEY_B), Action::Dir);
        assert_eq!(ws.cwd(), b"");
        // Formattazione: Select+B apre la conferma, B annulla, poi A conferma.
        assert_eq!(tap(&mut ws, &fs, KEY_SELECT | KEY_B), Action::FormatAsk);
        assert!(ws.confirming());
        assert_eq!(tap(&mut ws, &fs, KEY_B), Action::None);
        assert!(!ws.confirming());
        assert_eq!(tap(&mut ws, &fs, KEY_SELECT | KEY_B), Action::FormatAsk);
        assert_eq!(tap(&mut ws, &fs, KEY_A), Action::FormatSys);
        assert!(!ws.confirming());
        // Il file creato resta: la formattazione tocca solo lo spazio di sistema (fuori da questo test).
        assert!(fs.find_name(b"f0").is_some());
    }

    #[test]
    fn start_asks_for_create_and_names_child() {
        let fs = tree();
        let mut ws = Workspace::new();
        assert_eq!(ws.step(&fs, KEY_START), Action::Create);
        let (nm, n) = child_name(b"docs", 3);
        assert_eq!(&nm[..n], b"docs/f3");
        let (nm, n) = child_name(b"", 7);
        assert_eq!(&nm[..n], b"f7");
    }
}
