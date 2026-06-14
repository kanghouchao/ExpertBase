//! capture ドメイン層。素材タイプ判定など外部依存のない純ロジック。

/// 拡張子から素材タイプを判定する。未知は "file"。
pub fn kind_for_ext(ext: &str) -> &'static str {
  match ext.to_ascii_lowercase().as_str() {
    "md" | "markdown" | "txt" | "text" => "text",
    "pdf" => "pdf",
    "doc" | "docx" => "doc",
    "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" => "audio",
    "mp4" | "mov" | "mkv" | "webm" | "avi" => "video",
    "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => "image",
    _ => "file",
  }
}

/// 名前を (ステム, 拡張子) に分割する。
pub(crate) fn split_name(name: &str) -> (&str, Option<&str>) {
  match name.rsplit_once('.') {
    Some((stem, ext)) if !stem.is_empty() => (stem, Some(ext)),
    _ => (name, None),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn kind_for_ext_maps_known_extensions() {
    assert_eq!(kind_for_ext("pdf"), "pdf");
    assert_eq!(kind_for_ext("PNG"), "image");
    assert_eq!(kind_for_ext("mp3"), "audio");
    assert_eq!(kind_for_ext("md"), "text");
    assert_eq!(kind_for_ext("xyz"), "file");
  }
}
