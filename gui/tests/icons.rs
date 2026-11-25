// Icone: segni dentro il blocco 14×14, etichette note con glifo proprio.
use haiku_gui::icons::{cuts, BLOCK};

#[test]
fn cuts_stay_inside_block() {
    let (bx, by, bw, bh) = BLOCK;
    for l in ["FILE", "TERM", "CALC", "WEB", "EDIT", "SET", "ORA", "READ", "XXXX"] {
        for &(x, y, w, h) in cuts(l) {
            assert!(x >= bx && y >= by && x + w <= bx + bw && y + h <= by + bh, "{l}");
        }
    }
}

#[test]
fn known_labels_have_own_glyph() {
    assert_ne!(cuts("FILE"), cuts("XXXX"));
    assert_ne!(cuts("TERM"), cuts("CALC"));
}
