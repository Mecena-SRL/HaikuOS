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
    "T...=..........>",
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
    "<.........,,,,,T",
    "T.........,,,,,T",
    "T..,,,.........T",
    "T..,,,,,,,,,...T",
    "T..............T",
    "TTTTTTTTTTTTTTTT",
];

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

pub static NPCS: [Npc; 4] = [
    Npc { map: 0, x: 2, y: 2, kind: NpcKind::Nurse, text: "LE TUE CREATURE SONO GUARITE!" },
    Npc { map: 0, x: 11, y: 4, kind: NpcKind::Talk, text: "BENVENUTO A VERDEPOLI!|NELLE ERBE ALTE VIVONO CREATURE." },
    Npc { map: 0, x: 12, y: 9, kind: NpcKind::Talk, text: "VAI A EST NELLE ERBE PER LOTTARE." },
    Npc { map: 1, x: 9, y: 5, kind: NpcKind::Trainer, text: "EHI! SEI UN ALLENATORE?|TI SFIDO A UNA LOTTA!" },
];

pub fn map(m: u8) -> &'static [&'static str; 12] {
    if m == 0 { &TOWN } else { &ROUTE }
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
