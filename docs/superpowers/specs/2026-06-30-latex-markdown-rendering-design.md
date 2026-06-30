# Markdown LaTeX 表示対応設計

## 目的

共有 Markdown 表示で、`$\rightarrow$` のようなインライン LaTeX とブロック数式を表示できるようにする。

## 実装方針

- 既存の `react-markdown` に `remark-math` と `rehype-katex` を追加する。
- KaTeX の標準 CSS をアプリ全体で一度だけ読み込む。
- Markdown の保存形式と Workshop・Wiki の呼び出し側は変更しない。
- 任意 HTML は有効化せず、現在の安全な描画境界を維持する。

## 検証

- 共有 Markdown コンポーネントのテストで `$\rightarrow$` が KaTeX の数式として描画されることを確認する。
- ルートの lint と frontend build を実行する。

## 対象外

- 数式エディター、入力補完、独自 LaTeX コマンドは追加しない。
- MathJax や独自パーサーは導入しない。
