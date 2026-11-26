use translated::djb2;

#[test]
fn vuoto() {
    assert_eq!(djb2(b""), 5381);
}

#[test]
fn una_lettera() {
    assert_eq!(djb2(b"a"), 177670);
}

#[test]
fn overflow_64_bit() {
    assert_eq!(djb2(&[b'z'; 100]), 6986874207196428973);
}
