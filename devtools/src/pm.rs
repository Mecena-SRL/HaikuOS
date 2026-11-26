//! Package manager: indice, risoluzione delle dipendenze, installazione
//! atomica (tutto o nulla): ogni payload è verificato e messo in area di
//! staging prima di essere spostato nel posto definitivo.

use crate::{graph, pkg};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Index {
    #[serde(rename = "pkg")]
    pub pkgs: Vec<Entry>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Entry {
    pub name: String,
    pub version: String,
    pub sha256: String,
    #[serde(default)]
    pub deps: Vec<String>,
}

impl Index {
    /// Formato: `[[pkg]]` con name, version, sha256, deps.
    pub fn parse(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| e.to_string())
    }

    fn get(&self, name: &str) -> Option<&Entry> {
        self.pkgs.iter().find(|e| e.name == name)
    }

    /// Chiusura delle dipendenze di `root`, nell'ordine di installazione.
    pub fn resolve(&self, root: &[&str]) -> Result<Vec<String>, String> {
        let mut deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut todo: Vec<String> = root.iter().map(|s| s.to_string()).collect();
        while let Some(n) = todo.pop() {
            if deps.contains_key(&n) {
                continue;
            }
            let e = self.get(&n).ok_or_else(|| format!("pacchetto sconosciuto '{n}'"))?;
            todo.extend(e.deps.iter().cloned());
            deps.insert(n, e.deps.clone());
        }
        graph::build_order(&deps)
    }
}

/// Installa `names` (già in ordine) in `dest`. `fetch` restituisce il payload.
/// Se un solo pacchetto fallisce la verifica, non si installa nulla.
pub fn install(
    index: &Index,
    names: &[String],
    fetch: impl Fn(&str) -> Result<Vec<u8>, String>,
    dest: &Path,
) -> Result<usize, String> {
    let staging = dest.join(".staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let result = stage_all(index, names, &fetch, &staging).and_then(|()| {
        for n in names {
            fs::rename(staging.join(n), dest.join(n)).map_err(|e| format!("{n}: {e}"))?;
        }
        Ok(names.len())
    });
    let _ = fs::remove_dir_all(&staging);
    result
}

fn stage_all(
    index: &Index,
    names: &[String],
    fetch: &impl Fn(&str) -> Result<Vec<u8>, String>,
    staging: &Path,
) -> Result<(), String> {
    for n in names {
        let e = index.get(n).ok_or_else(|| format!("pacchetto sconosciuto '{n}'"))?;
        let payload = fetch(n)?;
        if !pkg::verify(&payload, &e.sha256) {
            return Err(format!("{n}: digest SHA-256 non corrisponde"));
        }
        fs::write(staging.join(n), &payload).map_err(|e| format!("{n}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDX: &str = r#"
[[pkg]]
name = "gfx"
version = "1"
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

[[pkg]]
name = "calc"
version = "1"
sha256 = "00"
deps = ["gfx"]
"#;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hdev-pm-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn resolves_deps_first() {
        let idx = Index::parse(IDX).unwrap();
        assert_eq!(idx.resolve(&["calc"]).unwrap(), ["gfx", "calc"]);
    }

    #[test]
    fn unknown_package_errors() {
        let idx = Index::parse(IDX).unwrap();
        assert!(idx.resolve(&["nope"]).unwrap_err().contains("nope"));
    }

    #[test]
    fn installs_verified_payload() {
        let idx = Index::parse(IDX).unwrap();
        let d = tmp("ok");
        let n = install(&idx, &["gfx".into()], |_| Ok(b"abc".to_vec()), &d).unwrap();
        assert_eq!(n, 1);
        assert_eq!(fs::read(d.join("gfx")).unwrap(), b"abc");
        assert!(!d.join(".staging").exists());
    }

    #[test]
    fn bad_digest_installs_nothing() {
        let idx = Index::parse(IDX).unwrap();
        let d = tmp("bad");
        // gfx è valido, calc ha digest sbagliato: nessuno dei due deve restare.
        let r = install(&idx, &["gfx".into(), "calc".into()], |_| Ok(b"abc".to_vec()), &d);
        assert!(r.unwrap_err().contains("calc"));
        assert!(!d.join("gfx").exists());
        assert!(!d.join("calc").exists());
    }
}
