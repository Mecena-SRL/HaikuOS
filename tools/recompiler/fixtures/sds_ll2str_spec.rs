use translated::ll2str;

fn conv(v: i64) -> ([u8; 32], usize) {
    let mut b = [0u8; 32];
    let n = ll2str(&mut b, v);
    (b, n)
}

#[test]
fn zero() {
    let (b, n) = conv(0);
    assert_eq!(&b[..n], b"0");
    assert_eq!(b[n], 0);
}

#[test]
fn positivo() {
    assert_eq!(&conv(12345).0[..5], b"12345");
}

#[test]
fn negativo() {
    let (b, n) = conv(-987);
    assert_eq!(&b[..n], b"-987");
}

#[test]
fn minimo() {
    let (b, n) = conv(i64::MIN);
    assert_eq!(&b[..n], b"-9223372036854775808");
}

#[test]
fn massimo() {
    let (b, n) = conv(i64::MAX);
    assert_eq!(&b[..n], b"9223372036854775807");
}
