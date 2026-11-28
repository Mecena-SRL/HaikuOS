//! Documenti di prova della suite office, inclusi nel kernel e decodificati all'avvio.
//! Nessun disco: la ISO li porta dentro. Ordine: docx, pdf, pptx, xlsx (come CSV).
use haiku_office::{formats, pdf, sheet::Sheet, slides, zip};

static DOCX: &[u8] = include_bytes!("../../../office/tests/fixtures/sample.docx");
static PDF: &[u8] = include_bytes!("../../../office/tests/fixtures/sample.pdf");
static PPTX: &[u8] = include_bytes!("../../../office/tests/fixtures/sample.pptx");
static XLSX: &[u8] = include_bytes!("../../../office/tests/fixtures/sample.xlsx");

const TXT: usize = 4096;
static mut OUT: [[u8; TXT]; 4] = [[0; TXT]; 4];
static mut XML: [u8; 16384] = [0; 16384];
static mut CBUF: [u8; 4096] = [0; 4096];
static mut SST: [u8; 4096] = [0; 4096];

/// Testo dei quattro documenti. Ogni slice è vuota se la decodifica fallisce.
pub fn load() -> [&'static [u8]; 4] {
    unsafe {
        let out = &mut *core::ptr::addr_of_mut!(OUT);
        let xml = &mut *core::ptr::addr_of_mut!(XML);
        let cbuf = &mut *core::ptr::addr_of_mut!(CBUF);
        let sst = &mut *core::ptr::addr_of_mut!(SST);
        let [t0, t1, t2, t3] = out.each_mut();

        let n0 = zip::extract(DOCX, b"word/document.xml", xml)
            .and_then(|n| formats::read_docx(&xml[..n], t0))
            .unwrap_or(0);
        let n1 = pdf::read_text(PDF, xml, cbuf, t1).unwrap_or(0);
        let n2 = slides::read_pptx(PPTX, xml, t2).unwrap_or(0);
        let mut sheet: Sheet<8, 6> = Sheet::new();
        let n3 = match sheet.read_xlsx(XLSX, xml, sst) {
            Ok(()) => sheet.write_csv(t3).unwrap_or(0),
            Err(_) => 0,
        };

        let o = &*core::ptr::addr_of!(OUT);
        [&o[0][..n0], &o[1][..n1], &o[2][..n2], &o[3][..n3]]
    }
}
