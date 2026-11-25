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
/// Metriche del font di sistema: 3×5 di default (GBA); feature `wide-font` per x86 (5×7).
#[cfg(not(feature = "wide-font"))]
pub const FONT_W: i32 = 3;
#[cfg(not(feature = "wide-font"))]
pub const FONT_H: i32 = 5;
#[cfg(feature = "wide-font")]
pub const FONT_W: i32 = 5;
#[cfg(feature = "wide-font")]
pub const FONT_H: i32 = 7;
pub const CONTENT_INSET: i32 = 4;
/// Pulsante HAIKUOS (apre il menu applicazioni): a sinistra della barra.
pub const MENU_W: i32 = 36;
/// Pulsanti finestra nella barra: primo a x=38, passo 42, larghi 40.
pub const TASK_X: i32 = 38;
pub const TASK_W: i32 = 40;
pub const TASK_PITCH: i32 = 42;
/// Orologio "HH:MM": 5 caratteri da 4 px meno lo spazio finale.
pub const CLOCK_W: i32 = 19;
/// Menu applicazioni: riga di 10 px (ricerca e voci), larghezza 100 px.
pub const MENU_ROW: i32 = 10;
pub const MENU_LIST_W: i32 = 100;

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

/// Pulsante riduci: a sinistra del massimizza, stesso passo di 2 px.
pub fn min_button(frame: Rect) -> Rect {
    let m = max_button(frame);
    Rect::new(m.x - CLOSE - 2, m.y, CLOSE, CLOSE)
}

/// Pulsante massimizza: a sinistra del chiudi.
pub fn max_button(frame: Rect) -> Rect {
    let c = close_button(frame);
    Rect::new(c.x - CLOSE - 2, c.y, CLOSE, CLOSE)
}

pub fn menu_button() -> Rect {
    Rect::new(0, 0, MENU_W, BAR_H)
}

/// Pulsante della finestra `i` nella barra (solo se entra prima dell'orologio).
pub fn task_button(i: usize) -> Rect {
    Rect::new(TASK_X + i as i32 * TASK_PITCH, 1, TASK_W, BAR_H - 2)
}

pub fn clock_x(screen_w: i32) -> i32 {
    screen_w - 2 - CLOCK_W
}

/// Menu applicazioni con `rows` voci più la riga di ricerca.
pub fn menu_rect(rows: usize) -> Rect {
    Rect::new(0, BAR_H, MENU_LIST_W, MENU_ROW * (rows as i32 + 1))
}

/// Corpo finestra: sotto la barra del titolo, dentro il bordo di 1 px.
pub fn body(frame: Rect) -> Rect {
    Rect::new(frame.x + 1, frame.y + 1 + TITLE_H, frame.w - 2, frame.h - 2 - TITLE_H)
}

/// Area dove il programma disegna: corpo con margine di 4 px.
pub fn content(frame: Rect) -> Rect {
    body(frame).inset(CONTENT_INSET)
}
