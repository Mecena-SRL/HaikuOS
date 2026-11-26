//! Lettura e scrittura del testo di docx (Word) e odt (OpenDocument). Solo `core`.
//! Lettura: solo il testo dei paragrafi, a capo LF. Stili, tabelle e immagini si perdono.
//! Scrittura: un documento minimo con paragrafi di solo testo.

use crate::zip;

fn tag(x: &[u8], i: usize) -> Option<(&[u8], bool, bool, usize)> {
    let j = i + x.get(i..)?.iter().position(|&b| b == b'>')?;
    let inner = &x[i + 1..j];
    let close = inner.first() == Some(&b'/');
    let selfclose = inner.last() == Some(&b'/');
    let body = if close { &inner[1..] } else { inner };
    let end = body.iter().position(|&b| b == b' ' || b == b'/' || b == b'\n' || b == b'\t').unwrap_or(body.len());
    Some((&body[..end], close, selfclose, j + 1))
}

fn push(out: &mut [u8], o: &mut usize, b: u8) -> Option<()> {
    *out.get_mut(*o)? = b;
    *o += 1;
    Some(())
}

/// Copia il testo di un nodo, decodificando le entità XML di base.
fn text(src: &[u8], out: &mut [u8], o: &mut usize) -> Option<()> {
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'&' {
            let rest = &src[i..];
            let semi = rest.iter().take(8).position(|&b| b == b';')?;
            let c = match &rest[1..semi] {
                b"amp" => b'&',
                b"lt" => b'<',
                b"gt" => b'>',
                b"quot" => b'"',
                b"apos" => b'\'',
                _ => b'?',
            };
            push(out, o, c)?;
            i += semi + 1;
        } else {
            push(out, o, src[i])?;
            i += 1;
        }
    }
    Some(())
}

/// Testo di `word/document.xml` (docx). Ritorna i byte scritti in `out`.
pub fn read_docx(xml: &[u8], out: &mut [u8]) -> Option<usize> {
    let (mut o, mut i, mut paras, mut in_t) = (0, 0, 0, false);
    while i < xml.len() {
        if xml[i] != b'<' {
            let next = i + xml[i..].iter().position(|&b| b == b'<').unwrap_or(xml.len() - i);
            if in_t {
                text(&xml[i..next], out, &mut o)?;
            }
            i = next;
            continue;
        }
        let (name, close, selfclose, next) = tag(xml, i)?;
        match name {
            b"w:p" if !close && !selfclose => {
                if paras > 0 {
                    push(out, &mut o, b'\n')?;
                }
                paras += 1;
            }
            b"w:t" if !selfclose => in_t = !close,
            b"w:tab" => push(out, &mut o, b'\t')?,
            b"w:br" => push(out, &mut o, b'\n')?,
            _ => {}
        }
        i = next;
    }
    Some(o)
}

/// Testo di `content.xml` (odt). Ritorna i byte scritti in `out`.
pub fn read_odt(xml: &[u8], out: &mut [u8]) -> Option<usize> {
    let (mut o, mut i, mut paras, mut depth) = (0, 0, 0, 0usize);
    while i < xml.len() {
        if xml[i] != b'<' {
            let next = i + xml[i..].iter().position(|&b| b == b'<').unwrap_or(xml.len() - i);
            if depth > 0 {
                text(&xml[i..next], out, &mut o)?;
            }
            i = next;
            continue;
        }
        let (name, close, selfclose, next) = tag(xml, i)?;
        match name {
            b"text:p" | b"text:h" if !close => {
                if !selfclose {
                    if paras > 0 {
                        push(out, &mut o, b'\n')?;
                    }
                    paras += 1;
                    depth += 1;
                }
            }
            b"text:p" | b"text:h" if close => depth = depth.saturating_sub(1),
            b"text:s" if depth > 0 => push(out, &mut o, b' ')?,
            b"text:tab" if depth > 0 => push(out, &mut o, b'\t')?,
            b"text:line-break" if depth > 0 => push(out, &mut o, b'\n')?,
            _ => {}
        }
        i = next;
    }
    Some(o)
}

const CT: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
const RELS: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
const MANIFEST: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\"><manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.text\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>";

/// Scrive testo (LF) come paragrafi XML. Il markup è fisso, il testo viene escapato.
fn paragraphs(text: &[u8], open: &[u8], close: &[u8], xml: &mut [u8]) -> Option<usize> {
    let mut o = 0;
    for line in text.split(|&b| b == b'\n') {
        for &b in open {
            push(xml, &mut o, b)?;
        }
        for &b in line {
            let esc: &[u8] = match b {
                b'&' => b"&amp;",
                b'<' => b"&lt;",
                b'>' => b"&gt;",
                _ => &[b],
            };
            for &c in esc {
                push(xml, &mut o, c)?;
            }
        }
        for &b in close {
            push(xml, &mut o, b)?;
        }
    }
    Some(o)
}

/// Scrive un docx minimo. `xml` è un buffer di lavoro, `out` riceve lo ZIP.
pub fn write_docx(text: &[u8], xml: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let mut o = 0;
    for &b in b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>" {
        push(xml, &mut o, b)?;
    }
    let n = paragraphs(text, b"<w:p><w:r><w:t xml:space=\"preserve\">", b"</w:t></w:r></w:p>", &mut xml[o..])?;
    o += n;
    for &b in b"<w:sectPr/></w:body></w:document>" {
        push(xml, &mut o, b)?;
    }
    zip::write(&[(b"[Content_Types].xml", CT), (b"_rels/.rels", RELS), (b"word/document.xml", &xml[..o])], out)
}

/// Scrive un odt minimo. `xml` è un buffer di lavoro, `out` riceve lo ZIP.
pub fn write_odt(text: &[u8], xml: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let head = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.2\"><office:body><office:text>";
    let tail = b"</office:text></office:body></office:document-content>";
    let mut o = 0;
    for &b in head {
        push(xml, &mut o, b)?;
    }
    o += paragraphs(text, b"<text:p>", b"</text:p>", &mut xml[o..])?;
    for &b in tail {
        push(xml, &mut o, b)?;
    }
    zip::write(&[(b"mimetype", b"application/vnd.oasis.opendocument.text"), (b"META-INF/manifest.xml", MANIFEST), (b"content.xml", &xml[..o])], out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odt_di_libreoffice() {
        let mut z = [0u8; 8192];
        let n = zip::extract(include_bytes!("../tests/fixtures/sample.odt"), b"content.xml", &mut z).unwrap();
        let mut t = [0u8; 256];
        let k = read_odt(&z[..n], &mut t).unwrap();
        assert_eq!(&t[..k], b"Haiku office prova\nseconda riga");
    }

    #[test]
    fn scrivi_docx_e_rileggi() {
        let mut xml = [0u8; 2048];
        let mut zbuf = [0u8; 4096];
        let n = write_docx(b"A & <B>\nriga due", &mut xml, &mut zbuf).unwrap();
        let mut doc = [0u8; 2048];
        let d = zip::extract(&zbuf[..n], b"word/document.xml", &mut doc).unwrap();
        let mut t = [0u8; 256];
        let k = read_docx(&doc[..d], &mut t).unwrap();
        assert_eq!(&t[..k], b"A & <B>\nriga due");
    }

    #[test]
    fn scrivi_odt_e_rileggi() {
        let mut xml = [0u8; 2048];
        let mut zbuf = [0u8; 4096];
        let n = write_odt(b"ciao\nmondo", &mut xml, &mut zbuf).unwrap();
        let mut doc = [0u8; 2048];
        let d = zip::extract(&zbuf[..n], b"content.xml", &mut doc).unwrap();
        let mut t = [0u8; 256];
        let k = read_odt(&doc[..d], &mut t).unwrap();
        assert_eq!(&t[..k], b"ciao\nmondo");
    }
}

#[cfg(test)]
mod tests_lo {
    use super::*;

    #[test]
    fn docx_di_libreoffice() {
        let mut z = [0u8; 8192];
        let n = zip::extract(include_bytes!("../tests/fixtures/sample.docx"), b"word/document.xml", &mut z).unwrap();
        let mut t = [0u8; 256];
        let k = read_docx(&z[..n], &mut t).unwrap();
        assert_eq!(&t[..k], b"Haiku office prova\nseconda riga");
    }
}
