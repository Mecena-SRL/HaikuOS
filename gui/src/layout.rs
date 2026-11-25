//! Misure in unità logiche = 1 px GBA (bibbia §5). Scala intera fuori da qui.

pub const BAR_H: i32 = 12;
pub const TITLE_H: i32 = 10;
pub const CLOSE: i32 = 8;
pub const ICON: i32 = 16;
pub const GRID_X: i32 = 8;
pub const GRID_Y: i32 = 24;
pub const PITCH_X: i32 = 48;
pub const PITCH_Y: i32 = 40;
pub const COLS: usize = 4;
pub const LABEL_DY: i32 = ICON + 2;
pub const UNDERLINE_DY: i32 = ICON + 1;
pub const FONT_W: i32 = 3;
pub const FONT_H: i32 = 5;
pub const CONTENT_INSET: i32 = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }

    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, self.w - 2 * d, self.h - 2 * d)
    }
}

/// Cella della griglia icone `i` (4 colonne, passo 48×40, inizio a (8, 24)).
pub fn icon_cell(i: usize) -> Rect {
    let (col, row) = ((i % COLS) as i32, (i / COLS) as i32);
    Rect::new(GRID_X + col * PITCH_X, GRID_Y + row * PITCH_Y, ICON, ICON)
}

/// Pulsante chiudi: 8×8, a 1 px dal bordo destro, nella barra del titolo.
pub fn close_button(frame: Rect) -> Rect {
    Rect::new(frame.x + frame.w - 2 - CLOSE, frame.y + 1, CLOSE, CLOSE)
}

pub fn title_bar(frame: Rect) -> Rect {
    Rect::new(frame.x + 1, frame.y + 1, frame.w - 2, TITLE_H)
}

/// Corpo finestra: sotto la barra del titolo, dentro il bordo di 1 px.
pub fn body(frame: Rect) -> Rect {
    Rect::new(frame.x + 1, frame.y + 1 + TITLE_H, frame.w - 2, frame.h - 2 - TITLE_H)
}

/// Area dove il programma disegna: corpo con margine di 4 px.
pub fn content(frame: Rect) -> Rect {
    body(frame).inset(CONTENT_INSET)
}
