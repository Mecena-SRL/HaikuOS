//! Test di DEFLATE, ZIP e testo di DOCX/ODT/ODS/XLSX su fixture reali (create con zipfile di Python).
//! Esegui: `cd sim && cargo test --test office`.

use haiku_alloc_sim::inflate::inflate;
use haiku_alloc_sim::office::read_text;
use haiku_alloc_sim::zip::{crc32, Archive, Error};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/office/");

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn inflate_vettori_zlib() {
    // Vettori prodotti da zlib (python), blocco Huffman fisso.
    let mut out = [0u8; 64];
    let n = inflate(&hex("cb48cdc9c957c8402701"), &mut out).unwrap();
    assert_eq!(&out[..n], b"hello hello hello hello");
    assert_eq!(inflate(&hex("0300"), &mut out).unwrap(), 0);
    let n = inflate(&hex("4b0400"), &mut out).unwrap();
    assert_eq!(&out[..n], b"a");
}

#[test]
fn inflate_blocco_stored() {
    let mut out = [0u8; 16];
    // BFINAL=1, BTYPE=00, LEN=5, NLEN=!5, "hello"
    let n = inflate(&[0x01, 0x05, 0x00, 0xfa, 0xff, b'h', b'e', b'l', b'l', b'o'], &mut out).unwrap();
    assert_eq!(&out[..n], b"hello");
}

#[test]
fn inflate_errori_senza_panico() {
    let mut out = [0u8; 16];
    assert_eq!(inflate(&[0x07], &mut out), Err(Error::BadData)); // BTYPE 11
    assert_eq!(inflate(&hex("cb48"), &mut out), Err(Error::Truncated));
    assert_eq!(inflate(&hex("cb48cdc9c957c8402701"), &mut [0u8; 4]), Err(Error::Full));
}

#[test]
fn crc32_vettore() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
}

#[test]
fn zip_elenca_e_verifica_crc() {
    let f = fixture("test.docx");
    let ar = Archive::open(&f).unwrap();
    let names: Vec<String> = ar
        .entries()
        .map(|e| String::from_utf8_lossy(e.unwrap().name).into_owned())
        .collect();
    assert_eq!(names, ["word/document.xml", "[Content_Types].xml"]);
    let e = ar.find("word/document.xml").unwrap();
    let mut buf = vec![0u8; e.usize_ as usize];
    let n = ar.extract(&e, &mut buf).unwrap();
    assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("Ciao"));

    // Corrompe un byte del contenuto compresso: CRC o decompressione devono fallire.
    let mut bad = f.clone();
    bad[50] ^= 0x55; // primo byte dei dati compressi di word/document.xml (intestazione di 30 + 17)
    let ar = Archive::open(&bad).unwrap();
    let r = ar.find("word/document.xml").and_then(|e| {
        let mut b = vec![0u8; e.usize_ as usize];
        ar.extract(&e, &mut b)
    });
    assert!(r.is_err());
}

#[test]
fn docx_testo() {
    assert_eq!(
        read_text(&fixture("test.docx")).unwrap(),
        "Ciao & benvenuto\nCittà è é\n\tRiga\ndue\n"
    );
}

#[test]
fn odt_testo() {
    assert_eq!(read_text(&fixture("test.odt")).unwrap(), "Titolo\nUno   due\nTre\tquattro <x>\n");
}

#[test]
fn ods_testo() {
    assert_eq!(read_text(&fixture("test.ods")).unwrap(), "Foglio1\n3\t\t\tOk\na\tb\n");
}

#[test]
fn xlsx_testo() {
    assert_eq!(
        read_text(&fixture("test.xlsx")).unwrap(),
        "primo\tsecondo & co\t42.5\ninline\tx\n"
    );
}

#[test]
fn deflate_dinamico_su_file_grande() {
    let t = read_text(&fixture("big.docx")).unwrap();
    assert_eq!(t.len(), 60_000);
    assert!(t.bytes().all(|b| b"abcdefgh lorem ipsum ".contains(&b)));
}

#[test]
fn file_troncati_non_fanno_panico() {
    for name in ["test.docx", "test.odt", "test.ods", "test.xlsx"] {
        let f = fixture(name);
        for cut in (0..f.len()).step_by(7) {
            let _ = read_text(&f[..cut]);
        }
    }
}

#[test]
fn pdf_testo_base() {
    let t = read_text(&fixture("sample.pdf")).unwrap();
    assert_eq!(t.trim(), "Ciao (mondo)\nRiga due\nTerza \\ riga\nTesto piano");
}

#[test]
fn scrittura_docx_odt_round_trip() {
    use haiku_alloc_sim::officewrite::{docx_vec, odt_vec};
    let t = "Ciao & <mondo>\n  due  spazi\nÈ tutto qui";
    let want = "Ciao & <mondo>\n  due  spazi\nÈ tutto qui\n";
    assert_eq!(read_text(&docx_vec(t).unwrap()).unwrap(), want);
    assert_eq!(read_text(&odt_vec(t).unwrap()).unwrap(), want);
}

#[test]
fn formato_sconosciuto() {
    let mut zf = Vec::new();
    {
        let mut w = std::io::Cursor::new(&mut zf);
        let mut z = zip_writer(&mut w);
        z.write(b"nulla");
    }
    assert!(read_text(&zf).is_err());
}

/// Scrittore ZIP minimo per il solo test di formato sconosciuto (voci stored).
struct Z<'a, W: std::io::Write>(&'a mut W);
fn zip_writer<'a, W: std::io::Write>(w: &'a mut W) -> Z<'a, W> {
    Z(w)
}
impl<W: std::io::Write> Z<'_, W> {
    fn write(&mut self, data: &[u8]) {
        let name = b"a.txt";
        let crc = crc32(data);
        let mut lh = Vec::new();
        lh.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        lh.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        lh.extend_from_slice(&crc.to_le_bytes());
        lh.extend_from_slice(&(data.len() as u32).to_le_bytes());
        lh.extend_from_slice(&(data.len() as u32).to_le_bytes());
        lh.extend_from_slice(&(name.len() as u16).to_le_bytes());
        lh.extend_from_slice(&0u16.to_le_bytes());
        lh.extend_from_slice(name);
        let mut cd = Vec::new();
        cd.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        cd.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        cd.extend_from_slice(&crc.to_le_bytes());
        cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
        cd.extend_from_slice(&[0; 12]);
        cd.extend_from_slice(&0u32.to_le_bytes());
        cd.extend_from_slice(name);
        let cd_off = lh.len() as u32;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        eocd.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        eocd.extend_from_slice(&(cd.len() as u32).to_le_bytes());
        eocd.extend_from_slice(&cd_off.to_le_bytes());
        eocd.extend_from_slice(&[0, 0]);
        let mut all = lh;
        all.extend_from_slice(data);
        all.extend_from_slice(&cd);
        all.extend_from_slice(&eocd);
        self.0.write_all(&all).unwrap();
    }
}
