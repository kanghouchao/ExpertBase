# アーキテクチャ観察リスト — Speculative 三件 + 確認済みの非摩擦

**日付**: 2026-07-12
**種別**: 観察メモ(実装しない。各項の発動条件が来たらカードを起こす)
**出自**: 2026-07-11 アーキテクチャ審査(`/improve-codebase-architecture`)。issue #40 から移設。語彙は `/codebase-design`: 深いモジュール · 縫い目 · アダプタ · テコ · 局所性。

---

## 使い方

- 次回審査が同じ観察を再発見するコストを省くための記録。
- **発動条件が来るまで手を付けない。** 先回りの実装・カード起こしはしない。
- 「確認済みの非摩擦」は審査済み・問題なしの結論＝再審査しない。

---

## 1. kb の読みコマンドが application を飛ばして index を直呼びする

- 6 つの読みコマンド(kb_list_entries / kb_search / kb_backlinks / kb_stats / kb_graph / kb_orphans、`src-tauri/src/kb/interface.rs:62-102`)が `index::…` を直接呼び、SQLite の行構造が三層を横断してフロントの DTO まで届いている。
- 欠けているのは縫い目一本であって余計なラッパーではない——ただしローカル単人アプリでは、読み取り DTO 層を今足すのは儀式にすぎないかもしれない。
- **発動条件**: 索引スキーマに手を入れるときにカードを起こす。

## 2. ModelDiscovery の縫い目

- モデル発見には実在の 2 変種(`src-tauri/src/agent/infrastructure/ollama.rs:33` / `openai_compat.rs:14` の各 `list_models`)が同じ骨格を共有するのに、ModelDiscovery の縫い目が無い。
- `OllamaModel` という名前は用途の半分に対して嘘をつく——openai_compat 側も `OllamaModel` を借りて返す(`openai_compat.rs:10` の import、`:53` では `tools: true` を固定で詰める)。
- 「アダプタが 2 つ＝本物の縫い目」がまさに当てはまる。既存の SearchBackend 縫い目(`src-tauri/src/workshop/infrastructure/web_search.rs`)を手本にすればよい。
- **発動条件**: 次に provider に手を入れるとき、ついでにやる。

## 3. workshop サイドバーの window イベント → typed pub/sub

- サイドバーとビューは window のグローバルイベント(裸の `Event`、文字列イベント名 `expertbase:workshop:*`)で通信している(`frontend/src/features/workshop/model/history.ts:24-40`)。縫い目ではあるが型が無く、テストは globalThis に EventTarget を敷く必要がある。
- モジュール内の typed pub/sub(workshop-run の subscribe/emit の流儀)に置き換えられる。
- **発動条件**: 次に workshop サイドバー通信に手を入れるとき。

---

## 2026-07-12 の照合

- 三件とも main(d232a69 時点)で依然として成立。発動条件はいずれも未到来(#37 graph reducer / #39 tools.rs 読み取り経路は三件のどれにも触れていない)。
- 行番号を現状へ更新済み(原 issue #40 の記載からの差分: history.ts 23-40 → 24-40)。
- 原 issue の「CustomEvent」は実際には裸の `Event` だった(本書で訂正)。

---

## 確認済みの非摩擦(審査結論、再審査しない)

- FSD の層別規律は実際に守られている(違反 import ゼロ)
- workshop-run は全リポジトリの深いモジュールの手本
- agent/domain と kb/domain の DDD は本物
- SearchBackend は既存の双アダプタによる本物の縫い目
- tools.rs の `_blocking` 分割は正当(rusqlite `Connection` は Sync ではないため、分割が削除テストを通る)
