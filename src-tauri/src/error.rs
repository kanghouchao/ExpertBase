//! IPC 境界の統一エラー型。前端の i18n key（例 "err.kb.nameRequired"）+ 補間パラメータを運ぶ。
//! `Display` は実装しない。`.to_string()` 経由でユーザー向け文言を暗黙に組み立てる経路を断つため。

use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
  pub code: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<BTreeMap<String, String>>,
}

impl AppError {
  pub fn code(code: &str) -> Self {
    Self { code: code.to_string(), params: None }
  }

  pub fn param(code: &str, key: &str, value: impl std::fmt::Display) -> Self {
    let mut params = BTreeMap::new();
    params.insert(key.to_string(), value.to_string());
    Self { code: code.to_string(), params: Some(params) }
  }

  pub fn params(code: &str, pairs: impl IntoIterator<Item = (&'static str, String)>) -> Self {
    let params: BTreeMap<String, String> =
      pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Self { code: code.to_string(), params: Some(params) }
  }

  /// 底層ライブラリ（io/sqlite/reqwest 等）の素通しエラー用。手書き文案ではないので個別コード化しない。
  pub fn generic(e: impl std::fmt::Display) -> Self {
    Self::param("err.generic", "detail", e)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn code_has_no_params() {
    let e = AppError::code("err.agent.cancelled");
    assert_eq!(e.code, "err.agent.cancelled");
    assert_eq!(e.params, None);
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v, serde_json::json!({ "code": "err.agent.cancelled" }));
  }

  #[test]
  fn param_carries_single_key() {
    let e = AppError::param("err.agent.network", "detail", "timeout");
    assert_eq!(e.code, "err.agent.network");
    assert_eq!(e.params.unwrap().get("detail"), Some(&"timeout".to_string()));
  }

  #[test]
  fn params_carries_multiple_keys() {
    let e = AppError::params(
      "err.agent.modelListFailed",
      [("status", "500".to_string()), ("detail", "boom".to_string())],
    );
    let params = e.params.unwrap();
    assert_eq!(params.get("status"), Some(&"500".to_string()));
    assert_eq!(params.get("detail"), Some(&"boom".to_string()));
  }

  #[test]
  fn generic_wraps_any_displayable_error_as_detail() {
    let e = AppError::generic("boom");
    assert_eq!(e.code, "err.generic");
    assert_eq!(e.params.unwrap().get("detail"), Some(&"boom".to_string()));
  }
}
