//! Presentazioni: formato nativo di testo (diapositive separate da una riga `---`).
//! Lettura di pptx: testo delle diapositive, in ordine di numero. Scrittura pptx: **rinviata**
//! (servono tema e master validi; da verificare con LibreOffice e PowerPoint).
//! Solo `core`, nessuna allocazione.

use crate::formats::{tag, text};
use crate::zip;

/// Numero di diapositive del testo nativo (una riga `---` separa due diapositive).
pub fn count(src: &[u8]) -> usize {
    1 + src.split(|&b| b == b'\n').filter(|l| *l == b"---").count()
}

/// Testo della diapositiva `idx`, senza le righe separatrici.
pub fn slide(src: &[u8], idx: usize) -> Option<&[u8]> {
    let mut start = 0;
    for n in 0..=idx {
        let mut p = start;
        loop {
            let e = src[p..].iter().position(|&b| b == b'\n').map_or(src.len(), |q| p + q);
            if &src[p..e] == b"---" {
                let end = if p > start { p - 1 } else { start };
                if n == idx {
                    return Some(&src[start..end]);
                }
                start = e + 1;
                break;
            }
            if e >= src.len() {
                return if n == idx { Some(&src[start..]) } else { None };
            }
            p = e + 1;
        }
    }
    None
}

/// Legge le diapositive di un pptx e le scrive nel formato nativo (`---` tra una e l'altra).
/// `scratch` riceve ogni file XML delle diapositive.
pub fn read_pptx(pptx: &[u8], scratch: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let mut o = 0;
    let mut k = 1;
    let mut name = [0u8; 32];
    while let Some(n) = slide_name(k, &mut name) {
        let Some(len) = zip::extract(pptx, &name[..n], scratch) else { break };
        if k > 1 {
            for &b in b"\n---\n" {
                crate::formats::push(out, &mut o, b)?;
            }
        }
        let xml = &scratch[..len];
        let mut i = 0;
        let mut para = 0;
        let mut in_t = false;
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
                b"a:p" if !close && !selfclose => {
                    if para > 0 {
                        crate::formats::push(out, &mut o, b'\n')?;
                    }
                    para += 1;
                }
                b"a:t" if !selfclose => in_t = !close,
                _ => {}
            }
            i = next;
        }
        k += 1;
    }
    if k == 1 {
        return None;
    }
    Some(o)
}

/// "ppt/slides/slideK.xml" in `buf`. Ritorna la lunghezza.
fn slide_name(k: usize, buf: &mut [u8; 32]) -> Option<usize> {
    if k > 999 {
        return None;
    }
    let head = b"ppt/slides/slide";
    buf[..head.len()].copy_from_slice(head);
    let mut o = head.len();
    let mut digits = [0u8; 4];
    let mut d = 0;
    let mut v = k;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        d += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        buf[o] = digits[d];
        o += 1;
    }
    for &b in b".xml" {
        buf[o] = b;
        o += 1;
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diapositive_nativo() {
        let src = b"Titolo\nriga\n---\nSecondo\n---\nTerzo";
        assert_eq!(count(src), 3);
        assert_eq!(slide(src, 0), Some(&b"Titolo\nriga"[..]));
        assert_eq!(slide(src, 1), Some(&b"Secondo"[..]));
        assert_eq!(slide(src, 2), Some(&b"Terzo"[..]));
        assert_eq!(slide(src, 3), None);
    }

    #[test]
    fn scrivi_pptx_e_rileggi() {
        let mut xml = [0u8; 16384];
        let mut z = [0u8; 16384];
        let n = write_pptx(b"Titolo & <x>\nriga\n---\nSecondo", &mut xml, &mut z).unwrap();
        let mut scratch = [0u8; 8192];
        let mut out = [0u8; 256];
        let k = read_pptx(&z[..n], &mut scratch, &mut out).unwrap();
        assert_eq!(&out[..k], "Titolo & <x>\nriga\n---\nSecondo".as_bytes());
    }

    #[test]
    fn pptx_di_pandoc() {
        let mut scratch = [0u8; 8192];
        let mut out = [0u8; 1024];
        let n = read_pptx(include_bytes!("../tests/fixtures/sample.pptx"), &mut scratch, &mut out).unwrap();
        assert_eq!(&out[..n], "Titolo\nriga uno\nriga due\n---\nSecondo\nTesto & fine".as_bytes());
    }
}

const NS: &[u8] = b"xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"";
const HEAD: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const GRP: &[u8] = b"<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>";
const REL_NS: &[u8] = b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">";
const REL_LAYOUT: &[u8] = b"<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>";
const THEME: &[u8] = b"<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"Haiku\"><a:themeElements><a:clrScheme name=\"Haiku\"><a:dk1><a:srgbClr val=\"000000\"/></a:dk1><a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"1F1F1F\"/></a:dk2><a:lt2><a:srgbClr val=\"EEEEEE\"/></a:lt2><a:accent1><a:srgbClr val=\"3366CC\"/></a:accent1><a:accent2><a:srgbClr val=\"DC3912\"/></a:accent2><a:accent3><a:srgbClr val=\"FF9900\"/></a:accent3><a:accent4><a:srgbClr val=\"109618\"/></a:accent4><a:accent5><a:srgbClr val=\"990099\"/></a:accent5><a:accent6><a:srgbClr val=\"0099C6\"/></a:accent6><a:hlink><a:srgbClr val=\"0000FF\"/></a:hlink><a:folHlink><a:srgbClr val=\"800080\"/></a:folHlink></a:clrScheme><a:fontScheme name=\"Haiku\"><a:majorFont><a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme><a:fmtScheme name=\"Haiku\"><a:fillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln><a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln><a:ln w=\"19050\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>";

const SLIDE_NAMES: [&[u8]; MAX_SLIDES] = [b"ppt/slides/slide1.xml", b"ppt/slides/slide2.xml", b"ppt/slides/slide3.xml", b"ppt/slides/slide4.xml", b"ppt/slides/slide5.xml", b"ppt/slides/slide6.xml", b"ppt/slides/slide7.xml", b"ppt/slides/slide8.xml"];
const SLIDE_RELS: [&[u8]; MAX_SLIDES] = [b"ppt/slides/_rels/slide1.xml.rels", b"ppt/slides/_rels/slide2.xml.rels", b"ppt/slides/_rels/slide3.xml.rels", b"ppt/slides/_rels/slide4.xml.rels", b"ppt/slides/_rels/slide5.xml.rels", b"ppt/slides/_rels/slide6.xml.rels", b"ppt/slides/_rels/slide7.xml.rels", b"ppt/slides/_rels/slide8.xml.rels"];
const SLD_ID: [&[u8]; MAX_SLIDES] = [b"256", b"257", b"258", b"259", b"260", b"261", b"262", b"263"];
const SLIDE_RID: [&[u8]; MAX_SLIDES] = [b"rId11", b"rId12", b"rId13", b"rId14", b"rId15", b"rId16", b"rId17", b"rId18"];
/// Diapositive massime nel pptx scritto.
pub const MAX_SLIDES: usize = 8;

struct W<'a> {
    buf: &'a mut [u8],
    o: usize,
}

impl W<'_> {
    fn s(&mut self, b: &[u8]) -> Option<()> {
        self.buf.get_mut(self.o..self.o + b.len())?.copy_from_slice(b);
        self.o += b.len();
        Some(())
    }
    fn esc(&mut self, t: &[u8]) -> Option<()> {
        for &b in t {
            match b {
                b'&' => self.s(b"&amp;")?,
                b'<' => self.s(b"&lt;")?,
                b'>' => self.s(b"&gt;")?,
                _ => self.s(&[b])?,
            }
        }
        Some(())
    }
}

/// Scrive un pptx minimo (tema, master, layout e fino a 8 diapositive di testo).
/// `src` è il testo nativo; `xml` è il buffer di lavoro, `out` riceve lo ZIP.
pub fn write_pptx(src: &[u8], xml: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let n = count(src);
    if n > MAX_SLIDES {
        return None;
    }
    let mut w = W { buf: xml, o: 0 };
    let mut parts = [(0usize, 0usize); 3 + 3 + MAX_SLIDES * 2];
    // Pezzo n: start..end nel buffer. Ordine fisso: vedi `names` sotto.
    let mut np = 0;
    macro_rules! part {
        ($body:expr) => {{
            let st = w.o;
            $body?;
            parts[np] = (st, w.o);
            np += 1;
        }};
    }
    part!({
        w.s(HEAD)?;
        w.s(b"<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>")?;
        w.s(b"<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/><Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/><Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/><Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>")?;
        for k in 1..=n {
            w.s(b"<Override PartName=\"/ppt/slides/slide")?;
            w.s(&[b'0' + k as u8])?;
            w.s(b".xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>")?;
        }
        w.s(b"</Types>")
    });
    part!({
        w.s(HEAD)?;
        w.s(REL_NS)?;
        w.s(b"<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/></Relationships>")
    });
    part!({
        w.s(HEAD)?;
        w.s(b"<p:presentation ")?;
        w.s(NS)?;
        w.s(b"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst>")?;
        for k in 1..=n {
            w.s(b"<p:sldId id=\"")?;
            w.s(SLD_ID[k - 1])?;
            w.s(b"\" r:id=\"")?;
            w.s(SLIDE_RID[k - 1])?;
            w.s(b"\"/>")?;
        }
        w.s(b"</p:sldIdLst><p:sldSz cx=\"9144000\" cy=\"6858000\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>")
    });
    part!({
        w.s(HEAD)?;
        w.s(REL_NS)?;
        w.s(b"<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>")?;
        for k in 1..=n {
            w.s(b"<Relationship Id=\"")?;
            w.s(SLIDE_RID[k - 1])?;
            w.s(b"\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide")?;
            w.s(&[b'0' + k as u8])?;
            w.s(b".xml\"/>")?;
        }
        w.s(b"</Relationships>")
    });
    part!({
        w.s(HEAD)?;
        w.s(b"<p:sldMaster ")?;
        w.s(NS)?;
        w.s(b"><p:cSld><p:spTree>")?;
        w.s(GRP)?;
        w.s(b"</p:spTree></p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst></p:sldMaster>")
    });
    part!({
        w.s(HEAD)?;
        w.s(REL_NS)?;
        w.s(b"<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/></Relationships>")
    });
    part!({
        w.s(HEAD)?;
        w.s(b"<p:sldLayout ")?;
        w.s(NS)?;
        w.s(b" type=\"blank\" preserve=\"1\"><p:cSld name=\"Vuoto\"><p:spTree>")?;
        w.s(GRP)?;
        w.s(b"</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>")
    });
    part!({
        w.s(HEAD)?;
        w.s(REL_NS)?;
        w.s(b"<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/></Relationships>")
    });
    part!({ w.s(THEME) });
    for k in 1..=n {
        part!({
            w.s(HEAD)?;
            w.s(b"<p:sld ")?;
            w.s(NS)?;
            w.s(b"><p:cSld><p:spTree>")?;
            w.s(GRP)?;
            w.s(b"<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Testo\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"457200\" y=\"457200\"/><a:ext cx=\"8229600\" cy=\"5715000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr wrap=\"square\"/><a:lstStyle/>")?;
            let text = slide(src, k - 1)?;
            for line in text.split(|&b| b == b'\n') {
                if line.is_empty() {
                    w.s(b"<a:p><a:endParaRPr lang=\"it-IT\"/></a:p>")?;
                } else {
                    w.s(b"<a:p><a:r><a:rPr lang=\"it-IT\"/><a:t>")?;
                    w.esc(line)?;
                    w.s(b"</a:t></a:r></a:p>")?;
                }
            }
            w.s(b"</p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>")
        });
        part!({
            w.s(HEAD)?;
            w.s(REL_NS)?;
            w.s(REL_LAYOUT)?;
            w.s(b"</Relationships>")
        });
    }
    // Nomi nello stesso ordine dei pezzi.
    let mut names: [&[u8]; 3 + 3 + MAX_SLIDES * 2] = [b""; 3 + 3 + MAX_SLIDES * 2];
    let fixed: [&[u8]; 9] = [
        b"[Content_Types].xml",
        b"_rels/.rels",
        b"ppt/presentation.xml",
        b"ppt/_rels/presentation.xml.rels",
        b"ppt/slideMasters/slideMaster1.xml",
        b"ppt/slideMasters/_rels/slideMaster1.xml.rels",
        b"ppt/slideLayouts/slideLayout1.xml",
        b"ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        b"ppt/theme/theme1.xml",
    ];
    names[..9].copy_from_slice(&fixed);
    for k in 0..n {
        names[9 + 2 * k] = SLIDE_NAMES[k];
        names[10 + 2 * k] = SLIDE_RELS[k];
    }
    let buf = w.buf;
    let mut ents: [(&[u8], &[u8]); 9 + MAX_SLIDES * 2] = [(b"", b""); 9 + MAX_SLIDES * 2];
    for i in 0..np {
        let (s, e) = parts[i];
        ents[i] = (names[i], &buf[s..e]);
    }
    zip::write(&ents[..np], out)
}
