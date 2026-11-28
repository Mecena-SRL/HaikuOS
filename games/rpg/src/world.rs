//! Mappe a tile (ASCII 16x12), NPC e passaggi tra le mappe. `no_std`.
//! Legenda: `.` erba, `=` sentiero, `,` erba alta (incontri), `#` muro,
//! `T` albero, `H` casa, `>` `<` passaggio tra le mappe.

pub const W: i32 = 16;
pub const H: i32 = 12;
pub const START: (i32, i32) = (7, 8);

/// Mappa 0: Verdepoli (città).
pub static TOWN: [&str; 12] = [
    "TTTTTTTTTTTTTTTT",
    "T..HHHH.....TT.T",
    "T..HHHH........T",
    "T...=..........T",
    "T...=..........T",
    "T...=..........T",
    "T...===========>",
    "T...=..........T",
    "T...=....TTT...T",
    "T...=..........T",
    "T..............T",
    "TTTTTTTTTTTTTTTT",
];

/// Mappa 1: Percorso 1 (erba alta e un allenatore).
pub static ROUTE: [&str; 12] = [
    "TTTTTTTTTTTTTTTT",
    "T.,,,,,........T",
    "T.,,,,,,,,,,,,,T",
    "T........,,,,,,T",
    "T........,,,,,,T",
    "T.........,,,,,T",
    "<.........,,,,,>",
    "T.........,,,,,T",
    "T..,,,.........T",
    "T..,,,,,,,,,...T",
    "T..............T",
    "TTTTTTTTTTTTTTTT",
];

/// Mappa 2: Percorso 2 (più erba, secondo allenatore).
pub static ROUTE2: [&str; 12] = [
    "TTTTTTTTTTTTTTTT",
    "T.,,,,,,,,,,,,.T",
    "T.,,,,,.....,,,T",
    "T........,,,,,,T",
    "T.........,,,,,T",
    "T.........,,,,,T",
    "<..........,,,,T",
    "T.........,,,,,T",
    "T..,,,.........T",
    "T..,,,,,,,,,...T",
    "T..............T",
    "TTTTTTTTTTTTTTTT",
];

/// Passaggi tra le mappe: (mappa, x, y) -> (mappa, x, y).
pub const WARPS: [(u8, i32, i32, u8, i32, i32); 4] = [
    (0, 15, 6, 1, 1, 6),
    (1, 0, 6, 0, 14, 6),
    (1, 15, 6, 2, 1, 6),
    (2, 0, 6, 1, 14, 6),
];

pub fn warp(m: u8, x: i32, y: i32) -> Option<(u8, i32, i32)> {
    WARPS.iter().find(|w| w.0 == m && w.1 == x && w.2 == y).map(|w| (w.3, w.4, w.5))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NpcKind {
    Talk,
    Trainer,
    Nurse,
}

#[derive(Clone, Copy)]
pub struct Npc {
    pub map: u8,
    pub x: i32,
    pub y: i32,
    pub kind: NpcKind,
    /// Pagine separate da `|`.
    pub text: &'static str,
}

pub static NPCS: [Npc; 5] = [
    Npc { map: 0, x: 2, y: 2, kind: NpcKind::Nurse, text: "LE TUE CREATURE SONO GUARITE!" },
    Npc { map: 0, x: 11, y: 4, kind: NpcKind::Talk, text: "BENVENUTO A VERDEPOLI!|NELLE ERBE ALTE VIVONO CREATURE." },
    Npc { map: 0, x: 12, y: 9, kind: NpcKind::Talk, text: "AD EST UN ALLENATORE TI ASPETTA!" },
    Npc { map: 1, x: 9, y: 5, kind: NpcKind::Trainer, text: "EHI! SEI UN ALLENATORE?|TI SFIDO A UNA LOTTA!" },
    Npc { map: 2, x: 9, y: 5, kind: NpcKind::Trainer, text: "NON PASSI SENZA LOTTA!|SFIDAMI!" },
];

pub fn map(m: u8) -> &'static [&'static str; 12] {
    match m {
        0 => &TOWN,
        1 => &ROUTE,
        _ => &ROUTE2,
    }
}

pub fn tile(m: u8, x: i32, y: i32) -> char {
    if x < 0 || y < 0 || x >= W || y >= H {
        return '#';
    }
    map(m)[y as usize].as_bytes()[x as usize] as char
}

pub fn walkable(m: u8, x: i32, y: i32) -> bool {
    matches!(tile(m, x, y), '.' | '=' | ',' | '<' | '>')
}

pub fn npc_at(m: u8, x: i32, y: i32) -> Option<usize> {
    NPCS.iter().position(|n| n.map == m && n.x == x && n.y == y)
}
