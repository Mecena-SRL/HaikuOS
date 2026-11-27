//! Testi incorporati per le app di lettura: righe corte (al massimo 12 caratteri) per la finestra.

pub const TITLES: [&str; 2] = ["INTRO", "OFFICE"];

/// READ: presentazione di HaikuOS.
pub const INTRO: &[&str] = &[
    "HAIKUOS",
    "",
    "SISTEMA OPERATIVO",
    "MODULARE E LEGGERO",
    "",
    "KERNEL RUST NO_STD",
    "",
    "ARM64 CON UEFI",
    "E GOP",
    "",
    "TASTIERA E SERIALE",
    "",
    "APP: CALC, TERM",
    "ORA, SET, EDIT",
    "",
    "PROSSIMI PASSI",
    "WEB CON RETE",
    "FILE SYSTEM",
    "HARDWARE REALE",
];

/// DOC: documento di esempio, sola lettura.
pub const OFFICE: &[&str] = &[
    "DOCUMENTO",
    "",
    "QUESTO E UN",
    "TESTO DI PROVA",
    "PER IL VISUAL",
    "IZZATORE DI",
    "DOCUMENTI.",
    "",
    "SCORRI CON W E S.",
    "",
    "LE FORMATI OFFICE",
    "SONO DA FARE.",
];

pub const TEXTS: [&[&str]; 2] = [INTRO, OFFICE];
