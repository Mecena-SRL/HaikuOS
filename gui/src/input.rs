//! Tasti del GBA (desktop-concept §5). Il puntatore usa `Shell::click`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
    L,
    R,
}
