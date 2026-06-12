import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "ExpertBase — 个人知识工坊",
  description: "把你的专长，变成可对话的知识库。",
};

// 初回描画前に保存済みのテーマ・アクセント・言語を <html> に反映する。
const themeScript = `(function(){try{
  var e=document.documentElement;
  if(localStorage.getItem('eb_dark')==='1')e.classList.add('dark');
  var a=localStorage.getItem('eb_accent');
  e.setAttribute('data-accent',a==='amber'||a==='plum'||a==='indigo'||a==='terracotta'?a:'terracotta');
  var l=localStorage.getItem('eb_lang');e.setAttribute('lang',l==='en'?'en':l==='ja'?'ja':'zh');
}catch(_){}})();`;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="zh"
      data-accent="terracotta"
      suppressHydrationWarning
      className="h-full"
    >
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body className="grain h-full">{children}</body>
    </html>
  );
}
