//! Multi-page PDF document generation for Inskea.
//!
//! Produces compliant, self-contained PDF 1.4 documents containing one or more pages
//! with embedded raster or vector graphics. Supports arbitrary per-page paper dimensions
//! (e.g. ISO A2, A3, A4, B-series, or custom freehand sizes).

use std::io::Write;

/// A single page specification for the PDF builder.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPage {
    /// Page width in PDF points (1 pt = 1/72 inch).
    pub width_pt: f64,
    /// Page height in PDF points.
    pub height_pt: f64,
    /// Raster image pixel width.
    pub image_width_px: u32,
    /// Raster image pixel height.
    pub image_height_px: u32,
    /// JPEG image stream bytes.
    pub jpeg_data: Vec<u8>,
}

/// Convert CSS / Canvas pixels (at 96 DPI) to PDF points (at 72 DPI).
pub fn px_to_pt(px: f64) -> f64 {
    px * 72.0 / 96.0
}

/// Builds a valid multi-page PDF 1.4 document from a collection of [`PdfPage`]s.
pub fn build_pdf(pages: &[PdfPage], title: &str) -> Vec<u8> {
    if pages.is_empty() {
        return Vec::new();
    }

    let mut buf = Vec::new();
    let mut offsets = Vec::new();

    // 1. PDF Header
    buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let num_pages = pages.len();
    // Object numbering:
    // 1: Catalog
    // 2: Pages root
    // For each page i in 0..num_pages:
    //   page_obj_id = 3 + i * 3
    //   image_obj_id = 4 + i * 3
    //   contents_obj_id = 5 + i * 3
    let total_objs = 2 + num_pages * 3;

    // 2. Object 1: Catalog
    offsets.push(buf.len());
    let _ = write!(
        buf,
        "1 0 obj\n<<\n  /Type /Catalog\n  /Pages 2 0 R\n>>\nendobj\n"
    );

    // 3. Object 2: Pages root
    offsets.push(buf.len());
    let mut kids = String::new();
    for i in 0..num_pages {
        let page_id = 3 + i * 3;
        kids.push_str(&format!("{page_id} 0 R "));
    }
    let _ = write!(
        buf,
        "2 0 obj\n<<\n  /Type /Pages\n  /Kids [{kids}]\n  /Count {num_pages}\n>>\nendobj\n"
    );

    // 4. Page objects, Image objects, Contents objects
    for (i, page) in pages.iter().enumerate() {
        let page_id = 3 + i * 3;
        let image_id = 4 + i * 3;
        let contents_id = 5 + i * 3;

        // Page Object
        offsets.push(buf.len());
        let _ = write!(
            buf,
            "{page_id} 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /MediaBox [0 0 {:.2} {:.2}]\n  /Resources <<\n    /ProcSet [/PDF /ImageC]\n    /XObject <<\n      /Im1 {image_id} 0 R\n    >>\n  >>\n  /Contents {contents_id} 0 R\n>>\nendobj\n",
            page.width_pt, page.height_pt
        );

        // Image Object
        offsets.push(buf.len());
        let _ = write!(
            buf,
            "{image_id} 0 obj\n<<\n  /Type /XObject\n  /Subtype /Image\n  /Width {}\n  /Height {}\n  /ColorSpace /DeviceRGB\n  /BitsPerComponent 8\n  /Filter /DCTDecode\n  /Length {}\n>>\nstream\n",
            page.image_width_px, page.image_height_px, page.jpeg_data.len()
        );
        buf.extend_from_slice(&page.jpeg_data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        // Contents Object
        offsets.push(buf.len());
        let content_stream = format!(
            "q\n{:.2} 0 0 {:.2} 0 0 cm\n/Im1 Do\nQ\n",
            page.width_pt, page.height_pt
        );
        let _ = write!(
            buf,
            "{contents_id} 0 obj\n<<\n  /Length {}\n>>\nstream\n{content_stream}endstream\nendobj\n",
            content_stream.len()
        );
    }

    // 5. Cross-reference Table (XRef)
    let xref_offset = buf.len();
    let _ = write!(buf, "xref\n0 {}\n", total_objs + 1);
    buf.extend_from_slice(b"0000000000 65535 f \r\n");
    for offset in &offsets {
        let _ = write!(buf, "{:010} 00000 n \r\n", offset);
    }

    // 6. Trailer
    let sanitized_title = title.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
    let _ = write!(
        buf,
        "trailer\n<<\n  /Size {}\n  /Root 1 0 R\n  /Info <<\n    /Title ({sanitized_title})\n    /Producer (Inskea)\n    /Creator (Inskea)\n  >>\n>>\nstartxref\n{xref_offset}\n%%EOF\n",
        total_objs + 1
    );

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pages_yields_empty_pdf() {
        assert!(build_pdf(&[], "Empty").is_empty());
    }

    #[test]
    fn single_page_pdf_structure() {
        let page = PdfPage {
            width_pt: 595.28,
            height_pt: 841.89,
            image_width_px: 1190,
            image_height_px: 1684,
            jpeg_data: vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46],
        };
        let pdf = build_pdf(&[page], "Test A4 Page");
        assert!(!pdf.is_empty());

        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.starts_with("%PDF-1.4"));
        assert!(pdf_str.ends_with("%%EOF\n"));
        assert!(pdf_str.contains("/Type /Catalog"));
        assert!(pdf_str.contains("/Type /Pages"));
        assert!(pdf_str.contains("/Count 1"));
        assert!(pdf_str.contains("/MediaBox [0 0 595.28 841.89]"));
        assert!(pdf_str.contains("/Filter /DCTDecode"));
        assert!(pdf_str.contains("Test A4 Page"));
        assert!(pdf_str.contains("xref\n0 6\n"));
    }

    #[test]
    fn multi_page_pdf_contains_all_pages_and_correct_count() {
        let page1 = PdfPage {
            width_pt: 500.0,
            height_pt: 400.0,
            image_width_px: 1000,
            image_height_px: 800,
            jpeg_data: vec![1, 2, 3, 4],
        };
        let page2 = PdfPage {
            width_pt: 600.0,
            height_pt: 800.0,
            image_width_px: 1200,
            image_height_px: 1600,
            jpeg_data: vec![5, 6, 7, 8],
        };
        let pdf = build_pdf(&[page1, page2], "Multi Page Gallery");
        let pdf_str = String::from_utf8_lossy(&pdf);

        assert!(pdf_str.contains("/Count 2"));
        assert!(pdf_str.contains("/Kids [3 0 R 6 0 R ]"));
        assert!(pdf_str.contains("/MediaBox [0 0 500.00 400.00]"));
        assert!(pdf_str.contains("/MediaBox [0 0 600.00 800.00]"));
        assert!(pdf_str.contains("xref\n0 9\n"));
    }

    #[test]
    fn px_to_pt_conversion_ratio() {
        // 96 px = 72 pt = 1 inch
        assert_eq!(px_to_pt(96.0), 72.0);
        assert_eq!(px_to_pt(0.0), 0.0);
        assert_eq!(px_to_pt(192.0), 144.0);
    }
}
