//! Ordine di build: ordinamento topologico (Kahn) con rilevamento dei cicli.

use std::collections::{BTreeMap, BTreeSet};

/// `deps[p]` = pacchetti da cui `p` dipende. Restituisce un ordine in cui
/// ogni pacchetto viene dopo le sue dipendenze. Errore se una dipendenza
/// manca o se c'è un ciclo.
pub fn build_order(deps: &BTreeMap<String, Vec<String>>) -> Result<Vec<String>, String> {
    for (p, ds) in deps {
        for d in ds {
            if !deps.contains_key(d) {
                return Err(format!("{p}: dipendenza mancante '{d}'"));
            }
        }
    }
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut order = Vec::with_capacity(deps.len());
    while done.len() < deps.len() {
        let ready: Vec<&String> = deps
            .iter()
            .filter(|(p, ds)| !done.contains(*p) && ds.iter().all(|d| done.contains(d)))
            .map(|(p, _)| p)
            .collect();
        if ready.is_empty() {
            let left: Vec<&str> = deps
                .keys()
                .filter(|p| !done.contains(*p))
                .map(|s| s.as_str())
                .collect();
            return Err(format!("ciclo tra: {}", left.join(", ")));
        }
        for p in ready {
            done.insert(p.clone());
            order.push(p.clone());
        }
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(v: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        v.iter()
            .map(|(p, ds)| (p.to_string(), ds.iter().map(|d| d.to_string()).collect()))
            .collect()
    }

    #[test]
    fn orders_dependencies_first() {
        let o = build_order(&g(&[("calc", &["ui"]), ("ui", &["gfx"]), ("gfx", &[])])).unwrap();
        let pos = |n: &str| o.iter().position(|x| x == n).unwrap();
        assert!(pos("gfx") < pos("ui") && pos("ui") < pos("calc"));
    }

    #[test]
    fn detects_cycle() {
        let e = build_order(&g(&[("a", &["b"]), ("b", &["a"])])).unwrap_err();
        assert!(e.starts_with("ciclo"));
    }

    #[test]
    fn detects_missing_dep() {
        let e = build_order(&g(&[("a", &["zzz"])])).unwrap_err();
        assert!(e.contains("zzz"));
    }
}
