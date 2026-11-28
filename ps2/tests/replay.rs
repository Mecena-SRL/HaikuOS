//! Adattatore host: riproduce byte registrati come se venissero dalla porta 0x60.
//! Stesso decoder del kernel, senza hardware.
use haiku_ps2::{pump, ByteSource, Decoder, Ev, Nav};

struct Replay<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl ByteSource for Replay<'_> {
    fn next_byte(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied();
        self.pos += 1;
        b
    }
}

fn run(bytes: &[u8], mouse: bool) -> Vec<Ev> {
    let mut dec = Decoder::new();
    let mut src = Replay { bytes, pos: 0 };
    let mut out = Vec::new();
    pump(&mut dec, &mut src, mouse, |e| out.push(e));
    out
}

#[test]
fn tastiera_registrata() {
    // 'a', Shift premuto, '8' (dà '*'), Shift rilasciato, freccia su (E0 48).
    let evs = run(&[0x1E, 0x2A, 0x09, 0xAA, 0xE0, 0x48], false);
    assert_eq!(evs, vec![Ev::Char(b'a'), Ev::Char(b'*'), Ev::Nav(Nav::Up)]);
}

#[test]
fn mouse_registrato() {
    // Pacchetto da 3 byte: pulsante sinistro, dx=+2, dy=-2 (byte con segno: 0xFE).
    let evs = run(&[0x09, 0x02, 0xFE], true);
    assert_eq!(evs, vec![Ev::Mouse { dx: 2, dy: -2, left: true }]);
}

#[test]
fn sorgente_vuota_non_produce_eventi() {
    assert!(run(&[], false).is_empty());
}
