# 工坊の対話履歴 Implementation Plan

**Goal:** 工坊の対話をナレッジベース単位で JSONL に永続化し、履歴から復元・継続できるようにする。

**Architecture:** 1 会話を `<kb>/.expertbase/conversations/<id>.jsonl` に保存する。Rust が JSONL の読み書きと active KB の検証を担当し、フロントエンドは進行中の 1 ターンだけをモジュールストアで保持する。履歴は `/workshop?conversation=<id>` で選択する。

**Tech Stack:** Rust 2021、serde/serde_json、chrono、Tauri 2 IPC、Next.js 16、React 19、TypeScript、Bun test。

---

## 確定事項

- SQLite は対話履歴に使用しない。
- 会話は UTF-8 JSONL とし、1 行を meta または message イベントとして扱う。
- ユーザー送信時に会話を保存し、生成失敗時もユーザーメッセージを残す。
- ユーザーが生成を停止した場合、生成済みの本文・思考・ツール履歴を残す。
- 対話 ID は KB ごとの正整数とする。
- 履歴一覧は更新日時の降順、1 ページ 20 件とする。
- 1 件の壊れた JSONL は一覧から除外し、他の正常な履歴は表示する。
- 壊れた会話を ID 指定で取得した場合はエラーを返す。
- KB 切替後に、旧 KB の生成結果を新しい active KB へ保存しない。
- 削除、改名、検索、ピン留め、KB 横断履歴は対象外とする。

## クロスプラットフォーム方針

- 対象 OS は macOS、Windows、Linux とする。
- フロントエンドは KB パスを登録情報から得た不透明な識別文字列として扱う。
- JavaScript でパス区切り文字を分割・結合・正規化しない。
- Rust はファイル配置に `Path` と `PathBuf::join` を使用し、`/` や `\\` を手作業で連結しない。
- 保存要求の `kbPath` は、新しい書き込み先として利用せず、Rust が解決した active KB と一致するかの検証にだけ使う。
- JSONL ファイル名は OS 共通で安全な数値 ID と `.jsonl` だけで構成する。
- Windows 形式と POSIX 形式の `kbPath` がフロントエンドから保存要求へそのまま渡ることをテストする。

## データ形式

会話ファイルの先頭は meta 行とする。

```json
{"type":"meta","id":1,"title":"質問","created_at":"2026-06-30T01:00:00.000Z","source_ids":[]}
```

各メッセージは時刻と表示用の完全な状態を持つ。

```json
{"type":"msg","at":"2026-06-30T01:00:01.000Z","message":{"role":"user","text":"質問"}}
{"type":"msg","at":"2026-06-30T01:00:02.000Z","message":{"role":"ai","text":"回答","thinking":"...","tools":[]}}
```

素材が増えた場合は新しい meta 行を追記し、読み込み時は最後の meta を採用する。メッセージは既存件数より後ろだけを追記する。

## ファイル責務

- `src-tauri/src/workshop/domain.rs`
  - 対話、メッセージ、ツールイベント、一覧 DTO、タイトル規則。
- `src-tauri/src/workshop/infrastructure/history.rs`
  - JSONL の作成、追記、取得、ページング、破損ファイルの隔離。
- `src-tauri/src/workshop/application.rs`
  - 固定ページサイズと、保存前の active KB 一致検証。
- `src-tauri/src/workshop/interface.rs`
  - Tauri IPC の引数変換と blocking 処理の分離。
- `frontend/src/shared/api/tauri/client.ts`
  - Rust と一致する型、および `kbPath` を含む保存要求。
- `frontend/src/features/workshop/model/workshop-run.ts`
  - 進行中ターン、元 KB、会話 ID、部分生成結果の保持。
- `frontend/src/features/workshop/ui/workshop-view.tsx`
  - URL による復元、送信時保存、KB と会話 ID を組み合わせた実行表示。
- `frontend/src/features/workshop/ui/workshop-history-nav.tsx`
  - 先頭ページ、追加読み込み、折りたたみ、再読み込み。

## 実装手順

### Task 1: JSONL 永続化

1. domain と JSONL リポジトリの失敗テストを追加する。
2. 新規保存、追記、素材更新、取得、更新日時順ページングを実装する。
3. 壊れた JSONL を一覧で飛ばし、個別取得ではエラーになるテストを追加する。
4. 対象 Rust テストを実行する。

### Task 2: active KB 境界

1. 進行中ターンへ `kbPath` を保持する失敗テストを追加する。
2. POSIX 形式と Windows 形式の文字列が変更されず保存要求へ渡ることを確認する。
3. Rust で active KB と要求元 `kbPath` の不一致を拒否するテストを追加する。
4. `workshop_save_conversation` を application の検証済み保存へ委譲する。
5. UI は `kbPath + conversationId` が一致する実行だけを現在の会話として表示する。

### Task 3: 履歴 UI と実行ライフサイクル

1. サイドバーに KB 単位の履歴を表示する。
2. 履歴 URL からメッセージ、素材、ツール表示を復元する。
3. 送信時にユーザーメッセージを保存して会話 ID を確保する。
4. 生成成功時に AI 応答を追記する。
5. 停止時は部分結果を追記し、失敗時は保存済みユーザーメッセージを維持する。
6. Workshop 表示中の KB 切替では進行中実行を破棄する。
7. Workshop 表示外で KB が切り替わっても、Rust の active KB 検証で誤保存を防ぐ。

## 検証

```bash
bun test frontend/src/features/workshop/model/history.test.ts \
  frontend/src/features/workshop/model/workshop-run.test.ts
bun run test
bun run lint
bun run --cwd frontend build
git diff --check
```

手動確認:

1. 新規送信直後に履歴が作成される。
2. 成功応答、生成失敗、停止時の部分応答が再起動後も復元される。
3. 20 件を超える履歴で追加読み込みと折りたたみが動く。
4. macOS、Windows、Linux の各環境で KB ディレクトリ配下へ JSONL が作られる。
5. KB A で生成中に KB B へ切り替えても、結果が KB B に保存されない。
6. 壊れた JSONL が 1 件あっても、正常な履歴は表示される。

## 成功条件

- 対話履歴に SQLite ファイルを作成しない。
- 各 KB の `.expertbase/conversations` だけに対話 JSONL を保存する。
- KB の切替、同じ会話 ID、OS ごとのパス表現で履歴が混ざらない。
- 破損した 1 会話が正常な履歴一覧を停止させない。
- Rust テスト、Bun テスト、lint、production build、diff check が成功する。
