//! Gestore dei font (prototipo host). Programma separato: usa il registro e il lettore
//! OpenType del simulatore. Vedi `docs/font-manager.md`.
//! Comandi, una riga ciascuno (senza persistenza tra esecuzioni):
//! `install <file>`, `list`, `preview <nome> <testo>`, `activate <nome>`,
//! `deactivate <nome>`, `system <nome>`, `collection <nome>`, `add <indice> <nome>`.

use haiku_alloc_sim::fonts::{FontError, FontId, FontManager, Format};
use haiku_alloc_sim::opentype::{self, OtError};
use haiku_alloc_sim::raster;
use std::collections::HashMap;

/// Altezza di anteprima in pixel per em.
pub const PREVIEW_PX: f64 = 16.0;

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

/// Stato del programma: registro dei font e file OpenType caricati (per nome di famiglia).
#[derive(Debug, Default)]
pub struct Session {
    pub fm: FontManager,
    files: HashMap<String, Vec<u8>>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Lettore di file: in produzione legge il disco, nei test restituisce byte in memoria.
pub type Reader<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

/// Installa un file OpenType/TrueType: il nome è la famiglia letta dal file.
pub fn install_bytes(s: &mut Session, data: &[u8]) -> Result<String, AppError> {
    let info = opentype::parse(data).map_err(AppError::Parse)?;
    s.fm.install(&info.family, Format::Vector)?;
    s.files.insert(info.family.clone(), data.to_vec());
    Ok(info.family)
}

fn lookup(s: &Session, name: &str) -> Result<FontId, AppError> {
    s.fm.find(name).ok_or_else(|| AppError::UnknownName(name.to_string()))
}

fn index(s: &Session, t: &str) -> Result<usize, AppError> {
    let i: usize = t.parse().map_err(|_| AppError::BadIndex(t.to_string()))?;
    s.fm.collection(i).ok_or_else(|| AppError::BadIndex(t.to_string()))?;
    Ok(i)
}

fn need<'a>(v: Option<&'a str>, what: &'static str) -> Result<&'a str, AppError> {
    v.ok_or(AppError::MissingArg(what))
}

/// Esegue una riga di comando e restituisce il messaggio da mostrare.
pub fn exec(s: &mut Session, line: &str, read: Reader) -> Result<String, AppError> {
    let mut parts = line.trim().splitn(3, ' ');
    let cmd = parts.next().unwrap_or("");
    let a = parts.next().filter(|t| !t.is_empty());
    let rest = parts.next();
    match cmd {
        "" => Ok(String::new()),
        "install" => {
            let data = read(need(a, "file")?).map_err(AppError::Io)?;
            Ok(format!("installato: {}", install_bytes(s, &data)?))
        }
        "list" => {
            let mut out = Vec::new();
            for i in 0..64u8 {
                let id = FontId(i);
                let Some(name) = s.fm.name(id) else { break };
                let stato = if s.fm.is_active(id) { "attivo" } else { "inattivo" };
                let sys = if s.fm.system() == Some(id) { " (sistema)" } else { "" };
                out.push(format!("{i} {name} {stato}{sys}"));
            }
            Ok(out.join("\n"))
        }
        "preview" => {
            let name = need(a, "nome")?;
            let id = lookup(s, name)?;
            let sample = rest.unwrap_or("");
            s.fm.preview(id, sample)?;
            let data = s.files.get(name).ok_or_else(|| AppError::UnknownName(name.to_string()))?;
            let canvas = raster::render_text(data, sample, PREVIEW_PX).map_err(AppError::Parse)?;
            Ok(canvas.to_art().join("\n"))
        }
        "activate" => {
            let id = lookup(s, need(a, "nome")?)?;
            s.fm.activate(id)?;
            Ok("attivato".into())
        }
        "deactivate" => {
            let id = lookup(s, need(a, "nome")?)?;
            s.fm.deactivate(id)?;
            Ok("disattivato".into())
        }
        "system" => {
            let id = lookup(s, need(a, "nome")?)?;
            s.fm.set_system(id)?;
            Ok("font di sistema impostato".into())
        }
        "collection" => {
            let i = s.fm.create_collection(need(a, "nome")?)?;
            Ok(format!("collezione {i}"))
        }
        "add" => {
            let i = index(s, need(a, "indice")?)?;
            let id = lookup(s, need(rest, "nome")?)?;
            s.fm.add_to_collection(i, id)?;
            Ok("aggiunto".into())
        }
        other => Err(AppError::UnknownCommand(other.to_string())),
    }
}

