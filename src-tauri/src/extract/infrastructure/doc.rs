use std::path::Path;

/// デジタル PDF からテキストを抽出する。
/// pdf-extract は不正な PDF で panic することがあるため catch_unwind で保護する。
pub fn extract_pdf(path: &Path) -> Result<String, String> {
  let path = path.to_path_buf();
  std::panic::catch_unwind(move || pdf_extract::extract_text(&path))
    .map_err(|_| "PDF 的解析失败（文件可能损坏或为扫描件）".to_string())?
    .map_err(|e| e.to_string())
}

/// Word(.docx) からテキストを抽出する。
pub fn extract_docx(path: &Path) -> Result<String, String> {
  use dotext::MsDoc;
  use std::io::Read;

  let path = path.to_path_buf();
  std::panic::catch_unwind(move || -> Result<String, String> {
    let mut file = dotext::Docx::open(&path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    file.read_to_string(&mut text).map_err(|e| e.to_string())?;
    Ok(text)
  })
  .map_err(|_| "Word 文档解析失败".to_string())?
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extract_pdf_errors_gracefully_on_invalid_input() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("bad.pdf");
    std::fs::write(&p, b"not a real pdf").unwrap();
    // panic ではなく Err になること。
    assert!(extract_pdf(&p).is_err());
  }

  #[test]
  fn extract_docx_errors_gracefully_on_invalid_input() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("bad.docx");
    std::fs::write(&p, b"not a real docx").unwrap();
    assert!(extract_docx(&p).is_err());
  }
}
