//! Installazione atomica: verifica, poi attivazione, con rollback.
//! Nessuno stato parziale: se qualcosa fallisce, si torna allo stato precedente.

use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    Arm7tdmi,
    Arm64,
    X86_64,
}

#[derive(Clone, Copy, Debug)]
pub struct Component {
    pub name: &'static str,
    pub archs: &'static [Arch],
    /// Se true, un componente incompatibile blocca l'intera installazione.
    pub required: bool,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallError {
    RequiredIncompatible(&'static str),
    OutOfRam,
    ActivationFailed(&'static str),
}

/// Fase 1, solo lettura: sceglie i componenti compatibili. Non tocca nulla.
pub fn verify(arch: Arch, ram: usize, comps: &[Component]) -> Result<Vec<Component>, InstallError> {
    let mut chosen = Vec::new();
    let mut used = 0;
    for c in comps {
        if !c.archs.contains(&arch) {
            if c.required {
                return Err(InstallError::RequiredIncompatible(c.name));
            }
            continue; // opzionale e incompatibile: non viene caricato
        }
        used += c.bytes;
        if used > ram {
            return Err(InstallError::OutOfRam);
        }
        chosen.push(*c);
    }
    Ok(chosen)
}

/// Fasi 2 e 3: attiva in ordine. Se una attivazione fallisce, disattiva
/// quelle già attive in ordine inverso e restituisce l'errore.
/// Il chiamante aggiorna il suo stato solo se riceve `Ok`.
pub fn install(
    arch: Arch,
    ram: usize,
    comps: &[Component],
    mut activate: impl FnMut(&'static str) -> bool,
    mut deactivate: impl FnMut(&'static str),
) -> Result<Vec<&'static str>, InstallError> {
    let plan = verify(arch, ram, comps)?;
    let mut active: Vec<&'static str> = Vec::new();
    for c in &plan {
        if activate(c.name) {
            active.push(c.name);
        } else {
            for n in active.iter().rev() {
                deactivate(n);
            }
            return Err(InstallError::ActivationFailed(c.name));
        }
    }
    Ok(active)
}
