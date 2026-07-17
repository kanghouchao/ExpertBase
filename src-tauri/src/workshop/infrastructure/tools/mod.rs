//! workshop インフラ: Rig の `Tool` トレイト実装（KB 操作の AI ツール）。
//! search_kb は読み取り（FTS 検索）、write_entry は書き込み（application::confirm へ委譲）。
//! `Tool::call` は async だが sqlite/FS はブロッキングなので spawn_blocking で橋渡しし、
//! 読み取り経路は with_index で root から索引を一度だけ開く（`Connection` は Sync ではないため共有しない）。
//! with_index / resolve_entry は kb_read（読み取り側）と kb_write（書き込み側）の両方から共有される。

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

use crate::kb::index;

use super::confirm::ConfirmGate;
use super::web_search::BraveSearchBackend;

pub(crate) mod kb_read;
pub(crate) mod kb_write;
pub(crate) mod source;
pub(crate) mod web;

use self::kb_read::{ListKb, ReadEntry, SearchKb};
use self::kb_write::{DeleteEntry, UpdateEntry, WriteEntry};
use self::source::ReadSource;
use self::web::{FetchWeb, SearchWeb};

pub(crate) type UsedSources = Arc<Mutex<Vec<String>>>;

fn remember_source(used_sources: &UsedSources, source: &str) {
  let mut refs = used_sources.lock().unwrap_or_else(|e| e.into_inner());
  if !refs.iter().any(|item| item == source) {
    refs.push(source.to_string());
  }
}

/// 工作坊のツール一式を組んで汎用 `agent` へ注入するために返す。
/// tools 能力の無いモデル（`tools_capable == false`）には一切登録せず空を返す
/// （#41 受入条件「ツールを登録しない」。呼べないツールをリクエストに載せると Ollama が
/// エラーになる。明示発動＝# Activated Skills 注入だけが使える経路）。
/// tools 能力があれば read_source・list_kb・search_kb・search_web・read_entry・write_entry・
/// fetch_web を常に登録する。
/// used_sources は read/fetch で読んだ素材を write_entry が entry.sources に残すための共有状態。
/// brave_api_key は search_web の backend へだけ渡し、ツール出力には含めない。
/// gate は破壊的ツール（write_entry / update_entry / delete_entry）が実行前に
/// ユーザー確認を取るための確認ゲート。
/// `activate_skill` は発見済み技能が 1 件以上あるときだけ追加登録する。
/// `activated_skill_names` は前ターンまでに発動済みの技能名（フロント管理）。単発生成内の
/// 重複排除表 `activated_this_call` をこれで種付けし、既に本文が # Activated Skills に載っている
/// 技能をモデルが再度 `activate_skill` した場合は「既発動」通知を返す（本文の二重消費を避ける）。
pub(crate) fn build_toolset(
  root: &Path,
  sources: &[String],
  brave_api_key: String,
  gate: Arc<ConfirmGate>,
  skills: &[crate::plugin::Skill],
  tools_capable: bool,
  activated_skill_names: &[String],
) -> Vec<Box<dyn rig_core::tool::ToolDyn>> {
  if !tools_capable {
    return Vec::new();
  }
  let used_sources: UsedSources = Arc::new(Mutex::new(Vec::new()));
  let mut tools: Vec<Box<dyn rig_core::tool::ToolDyn>> = vec![
    Box::new(ReadSource { sources: sources.to_vec(), used_sources: used_sources.clone() }),
    Box::new(ListKb { root: root.to_path_buf() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(SearchWeb { backend: Arc::new(BraveSearchBackend::new(brave_api_key)) }),
    Box::new(ReadEntry { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), used_sources: used_sources.clone(), gate: gate.clone() }),
    Box::new(UpdateEntry { root: root.to_path_buf(), gate: gate.clone() }),
    Box::new(DeleteEntry { root: root.to_path_buf(), gate }),
    Box::new(FetchWeb { used_sources }),
  ];
  if !skills.is_empty() {
    tools.push(Box::new(crate::plugin::ActivateSkill {
      skills: skills.to_vec(),
      activated_this_call: Arc::new(Mutex::new(activated_skill_names.to_vec())),
    }));
  }
  tools
}

/// system プロンプトの # Tools 節本文を toolset の `definition()` から生成する。
/// ツールの契約文は definition() が唯一の真源＝散文との二重管理・分岐を構造的に排除する。
/// 形式: `- name(args): description` + 引数ごとの箇条書き（説明があるもののみ）。
pub(crate) async fn render_tools_section(toolset: &[Box<dyn rig_core::tool::ToolDyn>]) -> String {
  let mut lines = Vec::new();
  for tool in toolset {
    let def = tool.definition(String::new()).await;
    let params = ordered_params(&def.parameters);
    let names = params.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(", ");
    lines.push(format!("- {}({}): {}", def.name, names, def.description));
    for (name, desc) in &params {
      if !desc.is_empty() {
        lines.push(format!("  - {name}: {desc}"));
      }
    }
  }
  lines.join("\n")
}

/// JSON Schema の parameters から（引数名, 説明）を取り出す。serde_json の object は
/// 辞書順で反復されるため、宣言順を保つ `required` 配列を先頭に、残りの optional を後ろに置く。
fn ordered_params(schema: &serde_json::Value) -> Vec<(String, String)> {
  let Some(props) = schema.get("properties").and_then(|v| v.as_object()) else {
    return Vec::new();
  };
  let required: Vec<&str> = schema
    .get("required")
    .and_then(|v| v.as_array())
    .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
    .unwrap_or_default();
  let mut names: Vec<&str> = required.clone();
  names.extend(props.keys().map(String::as_str).filter(|k| !required.contains(k)));
  names
    .into_iter()
    .map(|name| {
      let desc = props
        .get(name)
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or_default();
      (name.to_string(), desc.to_string())
    })
    .collect()
}

/// search_kb / search_web の引数。弱いモデルが欠落させても落ちないよう default で緩く受ける。
#[derive(Deserialize)]
pub struct SearchArgs {
  #[serde(default)]
  query: String,
}

/// 索引を一度だけ開いて読み取り操作へ渡すヘルパー。索引を開けなかった
/// ときの「索引エラー → モデル向け文字列」整形をここへ一元化する
/// （操作側のエラー整形は各操作が持つ）。
fn with_index<T>(
  root: &Path,
  f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
  let conn = index::open_index(root).map_err(|e| format!("(index error: {e:?})"))?;
  f(&conn)
}

/// id（path / 正確な title）を索引で既存条目へ解決し、entries/*.md へ再検証した相対パスと
/// タイトルを返す（read_entry / update_entry / delete_entry 共用）。接続は呼び出し側の
/// with_index から借りる。失敗は全てモデル向け文字列（ループ継続）。
fn resolve_entry(conn: &rusqlite::Connection, id: &str) -> Result<(String, String), String> {
  let refs = index::list_entries(conn).map_err(|e| format!("(index error: {e:?})"))?;
  // ponytail: 線形探索。条目一覧はメモリに収まる規模なので専用 SQL は不要。
  let Some(hit) = refs.into_iter().find(|r| r.path == id || r.title == id) else {
    return Err(format!("(no entry found: {id})"));
  };
  // 索引由来のパスでも entries/*.md に限定して再検証する（索引が壊れていても越境を防ぐ）。
  let Ok(rel) = crate::kb::checked_kb_markdown_path(&hit.path, "entries") else {
    return Err(format!("(invalid entry path in index: {})", hit.path));
  };
  Ok((rel.to_string_lossy().into_owned(), hit.title))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::entry::{Entry, EntryMeta};
  use std::sync::{Arc, Mutex};

  /// 確認要求へ自動応答するゲート（approve = 許可 / 拒否）。確認そのものが主題でないテスト用。
  pub(super) fn auto_gate(approve: bool) -> Arc<ConfirmGate> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        if let crate::agent::StreamProgress::ConfirmRequest { id, .. } = event {
          crate::workshop::infrastructure::confirm::resolve(&pending, id, approve);
        }
      }
    });
    gate
  }

  /// 手動応答用の確認ゲート一式（gate / 確認要求の受信側 / 未応答表）を組む。
  /// 確認カードの内容や応答前の状態を検証する、rx から id を取って自分で resolve するテスト用
  /// （自動応答で足りるテストは auto_gate を使う）。
  pub(super) fn manual_gate() -> (
    Arc<ConfirmGate>,
    tokio::sync::mpsc::UnboundedReceiver<crate::agent::StreamProgress>,
    crate::workshop::infrastructure::confirm::PendingConfirms,
  ) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
    (gate, rx, pending)
  }

  pub(super) fn seed_entry(conn: &rusqlite::Connection, path: &str, title: &str, body: &str) {
    let entry = Entry {
      meta: EntryMeta {
        kind: "Entry".into(),
        title: title.into(),
        description: String::new(),
        cat: "x".into(),
        tags: vec![],
        sources: vec![],
        created: "2026-06-14".into(),
        updated: "2026-06-14".into(),
      },
      body: body.into(),
    };
    index::upsert_entry(conn, path, &entry).unwrap();
  }

  /// 更新対象の条目をファイル + 索引の両方へ植える（update/delete 系テスト用）。
  pub(super) fn seed_entry_file(root: &Path, rel: &str, title: &str, body: &str) {
    std::fs::create_dir_all(root.join("entries")).unwrap();
    let content = format!(
      "---\ntype: Entry\ntitle: {title}\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n{body}\n"
    );
    std::fs::write(root.join(rel), content).unwrap();
    let conn = index::open_index(root).unwrap();
    seed_entry(&conn, rel, title, body);
  }

  #[tokio::test]
  async fn tools_section_renders_each_definition_with_signature_and_params() {
    let tmp = tempfile::tempdir().unwrap();
    let toolset =
      build_toolset(
        tmp.path(),
        &["/abs/a.md".to_string()],
        String::new(),
        auto_gate(true),
        &[],
        true,
        &[],
      );

    let out = render_tools_section(&toolset).await;

    // 全ツールが definition() 由来の 1 行で並ぶ（名前 + 署名 + 説明）。
    assert!(out.contains("- read_source(id): Read the full text"), "was: {out}");
    assert!(out.contains("- list_kb(): List knowledge base entries"), "was: {out}");
    assert!(out.contains("- search_kb(query): "), "was: {out}");
    assert!(out.contains("- search_web(query): "), "was: {out}");
    assert!(out.contains("- read_entry(id): "), "was: {out}");
    assert!(out.contains("- update_entry(id, body): "), "was: {out}");
    assert!(out.contains("- delete_entry(id): "), "was: {out}");
    assert!(out.contains("- fetch_web(url): "), "was: {out}");
    // 署名は required の宣言順が先、optional（cat）が後ろ（serde_json は挿入順を保持しないため）。
    assert!(out.contains("- write_entry(title, body, cat): "), "was: {out}");
    // 引数の説明も definition() の parameters から箇条書きで並ぶ。
    assert!(
      out.contains("- body: Entry body in Markdown, using [[title]] links to related notes"),
      "was: {out}"
    );
  }

  #[tokio::test]
  async fn tool_definitions_carry_the_full_contract_text() {
    // 旧 prompt.rs の散文にだけあった契約（書き込み門控・確認ゲート・素材の扱い）が
    // definition() に一本化されている＝モデルが見る物語は一つ。
    let tmp = tempfile::tempdir().unwrap();
    let toolset =
      build_toolset(
        tmp.path(),
        &["/abs/a.md".to_string()],
        String::new(),
        auto_gate(true),
        &[],
        true,
        &[],
      );

    let out = render_tools_section(&toolset).await;

    // read_source: 頼まれない限り要約・書き換えしない。
    assert!(out.contains("Do not summarize or rewrite a source unless the user asks."), "was: {out}");
    // list_kb / search_kb / read_entry / search_web: 使いどころの案内。
    assert!(out.contains("Use it to get an overview"), "was: {out}");
    assert!(out.contains("avoid duplicates"), "was: {out}");
    assert!(out.contains("before answering questions about it or building on it"), "was: {out}");
    assert!(out.contains("before relying on or saving its content"), "was: {out}");
    // 書き込み門控: ユーザーが頼んだときだけ + 実行前確認 + 拒否時は再試行しない。
    assert!(out.contains("Call only when the user asks to save"), "was: {out}");
    assert!(out.contains("The user is asked to approve the save before it happens"), "was: {out}");
    assert!(out.contains("read the entry first and include everything that should remain"), "was: {out}");
    assert!(out.contains("The user is asked to approve the change before it happens"), "was: {out}");
    assert!(out.contains("breaks [[links]] pointing to the entry"), "was: {out}");
    assert!(out.contains("The user is asked to approve the deletion before it happens"), "was: {out}");
  }

  fn a_skill() -> crate::plugin::Skill {
    crate::plugin::Skill {
      name: "tea-brewing".to_string(),
      description: "緑茶の淹れ方を案内する".to_string(),
      body: "本文".to_string(),
      location: "/skills/tea-brewing/SKILL.md".to_string(),
      source: crate::plugin::SkillSource::Kb,
      has_scripts: false,
    }
  }

  #[tokio::test]
  async fn build_toolset_registers_activate_skill_when_tools_capable_and_skills_exist() {
    let tmp = tempfile::tempdir().unwrap();
    let skills = vec![a_skill()];
    let toolset =
      build_toolset(tmp.path(), &[], String::new(), auto_gate(true), &skills, true, &[]);

    let out = render_tools_section(&toolset).await;

    assert!(out.contains("- activate_skill(name): "), "was: {out}");
  }

  #[tokio::test]
  async fn build_toolset_returns_no_tools_for_non_tools_capable_models() {
    // #41 受入条件「tools 非対応モデル: ツールを登録しない」。空 toolset は rig が
    // リクエストの tools 欄ごと省略する(skip_serializing_if)ので Ollama 側も安全。
    let tmp = tempfile::tempdir().unwrap();
    let skills = vec![a_skill()];
    let toolset =
      build_toolset(tmp.path(), &[], String::new(), auto_gate(true), &skills, false, &[]);

    assert!(toolset.is_empty());
  }

  #[tokio::test]
  async fn build_toolset_omits_activate_skill_when_no_skills_discovered() {
    let tmp = tempfile::tempdir().unwrap();
    let toolset = build_toolset(tmp.path(), &[], String::new(), auto_gate(true), &[], true, &[]);

    let out = render_tools_section(&toolset).await;

    assert!(!out.contains("activate_skill"), "was: {out}");
  }

  #[tokio::test]
  async fn build_toolset_seeds_activate_skill_dedup_from_already_activated_names() {
    // 前ターンまでに発動済みの技能は、モデルが再度 activate_skill しても本文を出し直さない
    // （本文は既に # Activated Skills に載っている、確定した決定10の一行改善）。
    let tmp = tempfile::tempdir().unwrap();
    let skills = vec![a_skill()];
    let toolset = build_toolset(
      tmp.path(),
      &[],
      String::new(),
      auto_gate(true),
      &skills,
      true,
      &["tea-brewing".to_string()],
    );
    let tool = toolset.iter().find(|t| rig_core::tool::ToolDyn::name(t.as_ref()) == "activate_skill").unwrap();

    let out = rig_core::tool::ToolDyn::call(tool.as_ref(), r#"{"name":"tea-brewing"}"#.to_string())
      .await
      .unwrap();

    assert!(out.contains("already activated this turn: tea-brewing"), "was: {out}");
  }

  #[test]
  fn source_tracking_deduplicates_urls() {
    let used_sources = Arc::new(Mutex::new(Vec::new()));
    remember_source(&used_sources, "https://example.com/article");
    remember_source(&used_sources, "https://example.com/article");

    assert_eq!(
      *used_sources.lock().unwrap(),
      vec!["https://example.com/article".to_string()]
    );
  }
}
