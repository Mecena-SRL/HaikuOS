//! Manifest `haiku.toml` di un pacchetto.
//!
//! ```toml
//! [package]
//! name = "calc"
//! version = "0.1.0"
//! archs = ["gba", "x86_64"]   # architetture supportate (regola di portabilità)
//! deps = ["ui"]
//! ```

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Manifest {
    pub package: Package,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub archs: Vec<String>,
    #[serde(default)]
    pub deps: Vec<String>,
}

impl Manifest {
    pub fn parse(src: &str) -> Result<Self, String> {
        let m: Manifest = toml::from_str(src).map_err(|e| e.to_string())?;
        if m.package.name.is_empty() {
            return Err("name vuoto".into());
        }
        if m.package.archs.is_empty() {
            return Err(format!("{}: archs mancanti", m.package.name));
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal() {
        let m = Manifest::parse(
            "[package]\nname = \"calc\"\nversion = \"0.1.0\"\narchs = [\"gba\"]\ndeps = [\"ui\"]\n",
        )
        .unwrap();
        assert_eq!(m.package.name, "calc");
        assert_eq!(m.package.deps, ["ui"]);
    }

    #[test]
    fn rejects_missing_archs() {
        let e = Manifest::parse("[package]\nname = \"x\"\nversion = \"1\"\n").unwrap_err();
        assert!(e.contains("archs"));
    }
}
