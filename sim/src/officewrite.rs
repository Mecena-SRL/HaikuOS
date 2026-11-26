//! Scrittura di DOCX e ODT minimi (solo testo, un paragrafo per riga). No_std: XML in `scratch`,
//! archivio in `out`. Il wrapper `std` restituisce un `Vec<u8>`. Dettagli: docs/office-formats.md.

use crate::zip::{write_stored, Error, Out};

const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";

const DOCX_CT: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
const DOCX_RELS: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
const ODT_MIME: &[u8] = b"application/vnd.oasis.opendocument.text";
const ODT_MANIFEST: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\"><manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.text\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>";

/// Testo XML con `&`, `<`, `>` escapati. Con `odf` le spaziature multiple diventano `<text:s/>`.
fn escape(o: &mut Out, s: &str, odf: bool) -> Result<(), Error> {
    // In ODF uno spazio raw in testo tutto-spazi verrebbe scartato dal lettore: inizio = spazio speciale.
    let mut prev_space = odf;
    for ch in s.chars() {
        match ch {
            '&' => o.put(b"&amp;")?,
            '<' => o.put(b"&lt;")?,
            '>' => o.put(b"&gt;")?,
            ' ' if odf && prev_space => o.put(b"<text:s/>")?,
            '\t' if odf => o.put(b"<text:tab/>")?,
            '\t' => o.put(b"</w:t><w:tab/><w:t xml:space=\"preserve\">")?,
            _ => {
                let mut b = [0u8; 4];
                o.put(ch.encode_utf8(&mut b).as_bytes())?;
            }
        }
        prev_space = ch == ' ';
    }
    Ok(())
}

/// DOCX con un paragrafo per riga di `text`.
pub fn docx(text: &str, scratch: &mut [u8], out: &mut [u8]) -> Result<usize, Error> {
    let n = {
        let mut x = Out::new(scratch);
        x.put(XML.as_bytes())?;
        x.put(b"<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>")?;
        for line in text.split('\n') {
            x.put(b"<w:p><w:r><w:t xml:space=\"preserve\">")?;
            escape(&mut x, line, false)?;
            x.put(b"</w:t></w:r></w:p>")?;
        }
        x.put(b"</w:body></w:document>")?;
        x.n
    };
    write_stored(out, &[("[Content_Types].xml", DOCX_CT), ("_rels/.rels", DOCX_RELS), ("word/document.xml", &scratch[..n])])
}

/// ODT con un paragrafo per riga di `text`. `mimetype` è la prima voce, senza compressione.
pub fn odt(text: &str, scratch: &mut [u8], out: &mut [u8]) -> Result<usize, Error> {
    let n = {
        let mut x = Out::new(scratch);
        x.put(XML.as_bytes())?;
        x.put(b"<office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.2\"><office:body><office:text>")?;
        for line in text.split('\n') {
            x.put(b"<text:p>")?;
            escape(&mut x, line, true)?;
            x.put(b"</text:p>")?;
        }
        x.put(b"</office:text></office:body></office:document-content>")?;
        x.n
    };
    write_stored(
        out,
        &[
            ("mimetype", ODT_MIME),
            ("META-INF/manifest.xml", ODT_MANIFEST),
            ("content.xml", &scratch[..n]),
        ],
    )
}

#[cfg(feature = "std")]
pub fn docx_vec(text: &str) -> Result<Vec<u8>, Error> {
    let cap = text.len() * 8 + text.lines().count() * 120 + 4096;
    let (mut scratch, mut out) = (vec![0u8; cap], vec![0u8; cap * 2 + 4096]);
    let n = docx(text, &mut scratch, &mut out)?;
    out.truncate(n);
    Ok(out)
}

#[cfg(feature = "std")]
pub fn odt_vec(text: &str) -> Result<Vec<u8>, Error> {
    let cap = text.len() * 8 + text.lines().count() * 120 + 4096;
    let (mut scratch, mut out) = (vec![0u8; cap], vec![0u8; cap * 2 + 4096]);
    let n = odt(text, &mut scratch, &mut out)?;
    out.truncate(n);
    Ok(out)
}
