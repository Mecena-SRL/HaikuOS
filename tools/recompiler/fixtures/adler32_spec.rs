use translated::adler32;

#[test]
fn vuoto() {
    assert_eq!(adler32(1, b""), 1);
}

#[test]
fn wikipedia() {
    assert_eq!(adler32(1, b"Wikipedia"), 0x11e60398);
}

#[test]
fn lungo_con_modulo() {
    assert_eq!(adler32(1, &[255u8; 100000]), 345649196);
}
