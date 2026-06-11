import type { Metadata } from "next";
import { Newsreader, Hanken_Grotesk, Spline_Sans_Mono } from "next/font/google";
import "./globals.css";

const hanken = Hanken_Grotesk({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  variable: "--font-hanken",
  display: "swap",
});

const newsreader = Newsreader({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  style: ["normal", "italic"],
  variable: "--font-newsreader",
  display: "swap",
});

const splineMono = Spline_Sans_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  variable: "--font-spline-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "ExpertBase — 个人知识工坊",
  description: "把你的专长，变成可对话的知识库。",
};

// Applies stored theme/accent/language to <html> before paint, so colors never
// flash. Phase 1 still renders copy in zh until hydration; cookie-backed SSR
// should replace localStorage language reads when server data lands.
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
      className={`${hanken.variable} ${newsreader.variable} ${splineMono.variable} h-full`}
    >
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        {/* next/font can't subset CJK; load Noto Sans SC via Google Fonts for 中文/日本語 glyphs */}
        {/* eslint-disable-next-line @next/next/no-page-custom-font */}
        <link
          href="https://fonts.googleapis.com/css2?family=Noto+Sans+SC:wght@400;500;700&display=swap"
          rel="stylesheet"
        />
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body className="grain h-full">{children}</body>
    </html>
  );
}
