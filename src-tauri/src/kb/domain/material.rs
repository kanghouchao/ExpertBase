use serde::{Deserialize, Serialize};

use super::entry::split_frontmatter;

fn default_status() -> String {
  "pending".to_string()
}

/// 受信箱素材（inbox/*.md）の frontmatter。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialMeta {
  /// text/web/pdf/doc/audio/video/image
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default)]
  pub source: String,
  /// pending/processed
  #[serde(default = "default_status")]
  pub status: String,
  /// 添付（attachments/ への相対パス）。任意。
  #[serde(default)]
  pub attachment: String,
  #[serde(default)]
  pub captured_at: String,
}

/// 受信箱素材 = frontmatter + 本文（抽出テキストまたはユーザー説明文、任意）。
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
  pub meta: MaterialMeta,
  pub body: String,
}

/// 受信箱素材を直列化する（条目と同じフェンス規約）。
pub fn serialize_material(material: &Material) -> Result<String, String> {
  let yaml = serde_yaml::to_string(&material.meta).map_err(|e| e.to_string())?;
  Ok(format!("---\n{yaml}---\n\n{}", material.body))
}

/// 受信箱素材を解析する（条目と同じフェンス規約）。
pub fn parse_material(raw: &str) -> Result<Material, String> {
  let (yaml, body) = split_frontmatter(raw)?;
  let meta: MaterialMeta = serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
  Ok(Material { meta, body })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn material_round_trips() {
    let raw = "---\ntype: web\nsource: https://x\nstatus: pending\ncaptured_at: 2026-06-14T00:00:00Z\n---\n\n本文テキスト\n";
    let m = parse_material(raw).unwrap();
    assert_eq!(m.meta.kind, "web");
    assert_eq!(m.meta.status, "pending");
    let again = parse_material(&serialize_material(&m).unwrap()).unwrap();
    assert_eq!(again, m);
  }
}
