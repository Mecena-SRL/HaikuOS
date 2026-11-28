//! App di sistema di HaikuOS: browser e editor di script.
//! Solo logica: nessun heap, nessuna rete, capacità fisse a compile time.
#![cfg_attr(not(test), no_std)]

pub mod browser;
pub mod editor;
pub mod shell;

/// Voce del lanciatore e del terminale.
pub struct AppEntry {
    pub name: &'static str,
    pub command: &'static str,
    pub summary: &'static str,
}

pub const APPS: &[AppEntry] = &[
    AppEntry { name: "Browser", command: "browse", summary: "Pagine di testo offline" },
    AppEntry { name: "Editor script", command: "run", summary: "Esegue script (sottoinsieme shell)" },
];

/// Unica sorgente di pagine e script. Nessuna rete: il file system è l'unico accesso.
pub trait Store {
    fn file(&self, name: &str) -> Option<&'static str>;
    /// Passa a `out` il nome di ogni file (per `ls`). Default: nessun file.
    fn names(&self, _out: &mut dyn FnMut(&str)) {}
}

#[cfg(test)]
pub(crate) mod testing {
    use super::Store;

    pub struct Files(pub &'static [(&'static str, &'static str)]);

    impl Store for Files {
        fn file(&self, name: &str) -> Option<&'static str> {
            self.0.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
        }
    }
}
