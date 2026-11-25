//! Gestore dei font (prototipo host). Programma separato: usa il registro e il lettore
//! OpenType del simulatore. Vedi `docs/font-manager.md`.
//! Comandi, una riga ciascuno (senza persistenza tra esecuzioni):
//! `install <file>`, `list`, `preview <nome> <testo>`, `activate <nome>`,
//! `deactivate <nome>`, `system <nome>`, `collection <nome>`, `add <indice> <nome>`.

use haiku_alloc_sim::fonts::{FontError, FontManager, Format};
use haiku_alloc_sim::opentype::{self, OtError};

#[derive(Debug, PartialEq, Eq)]
pub enum AppError {
    Font(FontError),
    Parse(OtError),
    Io(String),
    UnknownCommand(String),
    MissingArg(&'static str),
    UnknownName(String),
    BadIndex(String),
}

impl From<FontError> for AppError {
    fn from(e: FontError) -> Self {
        AppError::Font(e)
    }
}

/// Lettore di file: in produzione legge il disco, nei test restituisce byte in memoria.
pub type Reader<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

/// Installa un file OpenType/TrueType: il nome è la famiglia letta dal file.
pub fn install_bytes(fm: &mut FontManager, data: &[u8]) -> Result<String, AppError> {
    let info = opentype::parse(data).map_err(AppError::Parse)?;
    fm.install(&info.family, Format::Vector)?;
    Ok(info.family)
}

fn lookup(fm: &FontManager, name: &str) -> Result<haiku_alloc_sim::fonts::FontId, AppError> {
    fm.find(name).ok_or_else(|| AppError::UnknownName(name.to_string()))
}

fn index(fm: &FontManager, s: &str) -> Result<usize, AppError> {
    let i: usize = s.parse().map_err(|_| AppError::BadIndex(s.to_string()))?;
    fm.collection(i).ok_or_else(|| AppError::BadIndex(s.to_string()))?;
    Ok(i)
}

fn need<'a>(v: Option<&'a str>, what: &'static str) -> Result<&'a str, AppError> {
    v.ok_or(AppError::MissingArg(what))
}

/// Esegue una riga di comando e restituisce il messaggio da mostrare.
pub fn exec(fm: &mut FontManager, line: &str, read: Reader) -> Result<String, AppError> {
    let mut parts = line.trim().splitn(3, ' ');
    let cmd = parts.next().unwrap_or("");
    let a = parts.next().filter(|s| !s.is_empty());
    let rest = parts.next();
    match cmd {
        "" => Ok(String::new()),
        "install" => {
            let path = need(a, "file")?;
            let data = read(path).map_err(AppError::Io)?;
            Ok(format!("installato: {}", install_bytes(fm, &data)?))
        }
        "list" => {
            let mut out = Vec::new();
            for i in 0..64u8 {
                let id = haiku_alloc_sim::fonts::FontId(i);
                let Some(name) = fm.name(id) else { break };
                let stato = if fm.is_active(id) { "attivo" } else { "inattivo" };
                let sys = if fm.system() == Some(id) { " (sistema)" } else { "" };
                out.push(format!("{i} {name} {stato}{sys}"));
            }
            Ok(out.join("\n"))
        }
        "preview" => {
            let id = lookup(fm, need(a, "nome")?)?;
            Ok(fm.preview(id, rest.unwrap_or(""))?)
        }
        "activate" => {
            let id = lookup(fm, need(a, "nome")?)?;
            fm.activate(id)?;
            Ok("attivato".into())
        }
        "deactivate" => {
            let id = lookup(fm, need(a, "nome")?)?;
            fm.deactivate(id)?;
            Ok("disattivato".into())
        }
        "system" => {
            let id = lookup(fm, need(a, "nome")?)?;
            fm.set_system(id)?;
            Ok("font di sistema impostato".into())
        }
        "collection" => {
            let i = fm.create_collection(need(a, "nome")?)?;
            Ok(format!("collezione {i}"))
        }
        "add" => {
            let i = index(fm, need(a, "indice")?)?;
            let id = lookup(fm, need(rest, "nome")?)?;
            fm.add_to_collection(i, id)?;
            Ok("aggiunto".into())
        }
        other => Err(AppError::UnknownCommand(other.to_string())),
    }
}
