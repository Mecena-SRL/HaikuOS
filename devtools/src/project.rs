//! Progetto dell'IDE: elenco file di una cartella, caricamento e salvataggio dei buffer.
//! Ignora `.git`, `target` e i file nascosti.

use crate::ide::Buffer;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct Project {
    root: PathBuf,
    files: Vec<PathBuf>,
}

impl Project {
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut files = Vec::new();
        walk(&root, &root, &mut files)?;
        files.sort();
        Ok(Project { root, files })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Percorsi relativi alla radice.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn load(&self, rel: &Path) -> io::Result<Buffer> {
        Ok(Buffer::new(&fs::read_to_string(self.root.join(rel))?))
    }

    pub fn save(&self, rel: &Path, buf: &Buffer) -> io::Result<()> {
        fs::write(self.root.join(rel), buf.text())
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out)?;
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_skips_and_roundtrips() {
        let d = std::env::temp_dir().join(format!("hdev-proj-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src")).unwrap();
        fs::create_dir_all(d.join("target")).unwrap();
        fs::create_dir_all(d.join(".git")).unwrap();
        fs::write(d.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(d.join("haiku.toml"), "").unwrap();
        fs::write(d.join("target/x"), "").unwrap();
        fs::write(d.join(".git/y"), "").unwrap();

        let p = Project::open(&d).unwrap();
        assert_eq!(p.files(), [PathBuf::from("haiku.toml"), PathBuf::from("src/main.rs")]);

        let mut b = p.load(Path::new("src/main.rs")).unwrap();
        b.newline();
        p.save(Path::new("src/main.rs"), &b).unwrap();
        assert_eq!(fs::read_to_string(d.join("src/main.rs")).unwrap(), "\nfn main() {}");
        let _ = fs::remove_dir_all(&d);
    }
}
