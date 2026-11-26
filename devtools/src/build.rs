//! Build runner: piano dei passi per un'architettura ed esecuzione in ordine.

use crate::manifest::Package;
use std::process::Command;

#[derive(Debug, PartialEq)]
pub struct Step {
    pub name: String,
    pub cmd: String,
}

/// Passi per `arch`, nell'ordine `order` (già topologico).
/// Scarta i pacchetti non supportati da `arch`. Errore se un pacchetto
/// tenuto dipende da uno scartato.
pub fn plan(pkgs: &[Package], order: &[String], arch: &str) -> Result<Vec<Step>, String> {
    let supported = |name: &str| {
        pkgs.iter()
            .find(|p| p.name == name)
            .map_or(false, |p| p.archs.iter().any(|a| a == arch))
    };
    let mut steps = Vec::new();
    for name in order {
        let pkg = pkgs
            .iter()
            .find(|p| &p.name == name)
            .ok_or_else(|| format!("pacchetto sconosciuto '{name}'"))?;
        if !supported(name) {
            continue;
        }
        if let Some(d) = pkg.deps.iter().find(|d| !supported(d)) {
            return Err(format!("{name}: dipende da '{d}', che non supporta {arch}"));
        }
        if let Some(cmd) = &pkg.build {
            steps.push(Step { name: name.clone(), cmd: cmd.clone() });
        }
    }
    Ok(steps)
}

/// Esegue i passi in ordine, si ferma al primo errore. Ritorna quanti sono riusciti.
pub fn run(steps: &[Step], mut exec: impl FnMut(&Step) -> Result<(), String>) -> Result<usize, String> {
    for s in steps {
        exec(s).map_err(|e| format!("{}: {e}", s.name))?;
    }
    Ok(steps.len())
}

/// Esecuzione reale con `sh -c`.
pub fn exec_shell(step: &Step) -> Result<(), String> {
    let st = Command::new("sh")
        .arg("-c")
        .arg(&step.cmd)
        .status()
        .map_err(|e| e.to_string())?;
    if st.success() { Ok(()) } else { Err(format!("uscita {st}")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, archs: &[&str], deps: &[&str], build: Option<&str>) -> Package {
        Package {
            name: name.into(),
            version: "0.1.0".into(),
            archs: archs.iter().map(|s| s.to_string()).collect(),
            deps: deps.iter().map(|s| s.to_string()).collect(),
            build: build.map(|s| s.to_string()),
        }
    }

    #[test]
    fn plans_only_supported_in_order() {
        let pkgs = [
            pkg("gfx", &["gba", "x86_64"], &[], Some("make gfx")),
            pkg("ui", &["x86_64"], &["gfx"], Some("make ui")),
            pkg("calc", &["gba", "x86_64"], &["gfx"], None),
        ];
        let order: Vec<String> = ["gfx", "ui", "calc"].iter().map(|s| s.to_string()).collect();
        let steps = plan(&pkgs, &order, "gba").unwrap();
        assert_eq!(steps, vec![Step { name: "gfx".into(), cmd: "make gfx".into() }]);
    }

    #[test]
    fn errors_when_kept_pkg_needs_dropped_dep() {
        let pkgs = [
            pkg("gfx", &["x86_64"], &[], Some("make")),
            pkg("calc", &["gba", "x86_64"], &["gfx"], Some("make calc")),
        ];
        let order: Vec<String> = ["gfx", "calc"].iter().map(|s| s.to_string()).collect();
        let e = plan(&pkgs, &order, "gba").unwrap_err();
        assert!(e.contains("gfx"));
    }

    #[test]
    fn run_stops_at_first_failure() {
        let steps = [
            Step { name: "a".into(), cmd: "".into() },
            Step { name: "b".into(), cmd: "".into() },
            Step { name: "c".into(), cmd: "".into() },
        ];
        let mut seen = 0;
        let r = run(&steps, |s| {
            seen += 1;
            if s.name == "b" { Err("boom".into()) } else { Ok(()) }
        });
        assert_eq!(seen, 2);
        assert_eq!(r.unwrap_err(), "b: boom");
    }
}
