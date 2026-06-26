"use client";

import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";

// Markdown を安全な React ノードへ描画する共通プリミティブ。AI 出力・KB 条目本文の
// 「表示」専用（保存・生成は常に Markdown のまま）。dangerouslySetInnerHTML は使わない＝
// モデル出力でも XSS にならない。各要素はアプリのデザイントークンで体裁を合わせる。
const components: Components = {
  h1: ({ children }) => (
    <h1 className="mt-4 mb-2 font-serif text-[21px] font-semibold text-ink first:mt-0">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="mt-4 mb-2 font-serif text-[18px] font-semibold text-ink first:mt-0">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="mt-3 mb-1.5 font-serif text-[16px] font-semibold text-ink first:mt-0">
      {children}
    </h3>
  ),
  p: ({ children }) => <p className="my-2 leading-relaxed first:mt-0 last:mb-0">{children}</p>,
  ul: ({ children }) => <ul className="my-2 list-disc pl-5 leading-relaxed">{children}</ul>,
  ol: ({ children }) => <ol className="my-2 list-decimal pl-5 leading-relaxed">{children}</ol>,
  li: ({ children }) => <li className="my-0.5">{children}</li>,
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="text-ai underline underline-offset-2 hover:opacity-80"
    >
      {children}
    </a>
  ),
  strong: ({ children }) => <strong className="font-semibold text-ink">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
  blockquote: ({ children }) => (
    <blockquote className="my-2 border-l-2 border-ai-soft pl-3 text-ink-muted">{children}</blockquote>
  ),
  hr: () => <hr className="my-3 border-line" />,
  code: ({ className, children }) => {
    // フェンス付きコードブロックは language-* が付く＝そのまま pre 内へ。
    // それ以外はインラインコードとして背景＋パディングを付ける。
    const isBlock = /language-/.test(className ?? "");
    return isBlock ? (
      <code className={`${className} font-mono`}>{children}</code>
    ) : (
      <code className="rounded bg-surface-2 px-1 py-0.5 font-mono text-[0.9em] text-ink">
        {children}
      </code>
    );
  },
  pre: ({ children }) => (
    <pre className="my-2.5 overflow-auto rounded-lg border border-line bg-surface-2 p-3 font-mono text-[12.5px] leading-relaxed text-ink-soft">
      {children}
    </pre>
  ),
  table: ({ children }) => (
    <div className="my-2.5 overflow-auto">
      <table className="w-full border-collapse text-[13.5px]">{children}</table>
    </div>
  ),
  th: ({ children }) => (
    <th className="border border-line bg-surface-2 px-2.5 py-1.5 text-left font-semibold text-ink">
      {children}
    </th>
  ),
  td: ({ children }) => <td className="border border-line px-2.5 py-1.5 text-ink-soft">{children}</td>,
};

export function Markdown({ children, className }: { children: string; className?: string }) {
  return (
    <div className={className}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {children}
      </ReactMarkdown>
    </div>
  );
}
