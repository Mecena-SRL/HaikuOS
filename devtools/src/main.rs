//! `hdev build <arch> <dir>...`: legge `haiku.toml` di ogni dir, compila in ordine.

use devtools::{build, graph, manifest::{Manifest, Package}};
use std::collections::BTreeMap;

const USO: &str = "uso: hdev build <arch> <dir>...";

fn main() {
    if let Err(e) = real_main() {
        eprintln!("hdev: {e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cmd, arch, dirs @ ..] = args.as_slice() else {
        return Err(USO.into());
    };
    if cmd != "build" || dirs.is_empty() {
        return Err(USO.into());
    }
    let mut pkgs: Vec<Package> = Vec::new();
    for d in dirs {
        let src = std::fs::read_to_string(format!("{d}/haiku.toml")).map_err(|e| format!("{d}: {e}"))?;
        pkgs.push(Manifest::parse(&src)?.package);
    }
    let deps: BTreeMap<String, Vec<String>> =
        pkgs.iter().map(|p| (p.name.clone(), p.deps.clone())).collect();
    let order = graph::build_order(&deps)?;
    let steps = build::plan(&pkgs, &order, arch)?;
    let n = build::run(&steps, build::exec_shell)?;
    println!("hdev: {n} passi completati per {arch}");
    Ok(())
}
