//! Sottoinsieme dei comandi del terminale. Usato dal terminale e dagli script.

use crate::{editor, Store, APPS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellError {
    Unknown,
    MissingArg,
    NotFound,
    /// Uno script non può lanciarne un altro.
    NestedRun,
}

/// Esegue una riga. `depth` è 0 dal terminale, 1 dentro uno script.
pub fn exec(line: &str, store: &dyn Store, out: &mut dyn FnMut(&str), depth: u8) -> Result<(), ShellError> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(());
    }
    let (cmd, arg) = match line.find(' ') {
        Some(i) => (&line[..i], line[i + 1..].trim()),
        None => (line, ""),
    };
    match cmd {
        "help" => {
            out("help apps echo browse run");
            Ok(())
        }
        "apps" => {
            for app in APPS {
                out(app.name);
            }
            Ok(())
        }
        "echo" => {
            out(arg);
            Ok(())
        }
        "browse" => {
            let body = file(store, arg)?;
            for l in body.lines() {
                out(l);
            }
            Ok(())
        }
        "run" => {
            if depth > 0 {
                return Err(ShellError::NestedRun);
            }
            let body = file(store, arg)?;
            editor::run_lines(store, body.lines(), out).map_err(|(_, e)| e)
        }
        _ => Err(ShellError::Unknown),
    }
}

fn file(store: &dyn Store, name: &str) -> Result<&'static str, ShellError> {
    if name.is_empty() {
        return Err(ShellError::MissingArg);
    }
    store.file(name).ok_or(ShellError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Files;

    static STORE: Files = Files(&[
        ("script", "# prova\necho uno\n\necho due"),
        ("loop", "run script"),
        ("pagina", "# Titolo\n=> x"),
    ]);

    fn run(line: &str) -> (Result<(), ShellError>, std::vec::Vec<std::string::String>) {
        let mut out = std::vec::Vec::new();
        let r = exec(line, &STORE, &mut |s: &str| out.push(s.into()), 0);
        (r, out)
    }

    #[test]
    fn echo_and_apps() {
        assert_eq!(run("echo ciao  mondo").1, ["ciao  mondo"]);
        assert_eq!(run("apps").1, ["Browser", "Editor script"]);
    }

    #[test]
    fn browse_prints_page() {
        assert_eq!(run("browse pagina").1, ["# Titolo", "=> x"]);
        assert_eq!(run("browse").0, Err(ShellError::MissingArg));
        assert_eq!(run("browse nope").0, Err(ShellError::NotFound));
    }

    #[test]
    fn run_script_and_reject_nesting() {
        assert_eq!(run("run script").1, ["uno", "due"]);
        assert_eq!(run("run loop").0, Err(ShellError::NestedRun));
    }

    #[test]
    fn unknown_and_comments() {
        assert_eq!(run("rm -rf /").0, Err(ShellError::Unknown));
        assert_eq!(run("# solo commento").0, Ok(()));
    }
}
