// English / Japanese overrides for structured data (the prototype's data_en.jsx).
// User-authored knowledge prose stays as written; only labels/summaries swap.
// `L()` picks an override for the active language, falling back to the base
// (zh) field. Phase 1 only needs the `lint` kind — extend per view as needed.

import type { Lang } from "@/lib/i18n/dictionaries";

type Override = Record<string, Record<string, Record<string, string>>>;

const EN: Override = {
  lint: {
    l1: {
      title: "Orphan entries",
      detail:
        "“Yixing care” and “Cold brew” have no bidirectional links — suggest linking them to related ware/brewing entries.",
    },
    l2: {
      title: "Thin content",
      detail:
        "“Roasting” is only 640 words with a single citation — suggest adding rest cycles and roast levels.",
    },
    l3: {
      title: "Likely duplicate",
      detail:
        "The oxidation passages in “Tea polyphenols” and “Kill-green” overlap heavily — merge or cross-reference.",
    },
    l4: {
      title: "Stale",
      detail:
        "“Cold brew” hasn’t been updated in a month, while member questions about it are rising.",
    },
  },
  plugin: {
    p1: {
      name: "Whisper Transcription",
      blurb:
        "Automatically transcribes uploaded audio and video into timestamped text, with mixed Chinese-English and dialect recognition.",
    },
    p2: {
      name: "Local Vector Store",
      blurb:
        "Vectorizes Wiki entries locally for offline retrieval. Data never leaves this machine.",
    },
    p3: {
      name: "LINE Bot",
      blurb:
        "Publish the Wiki as a LINE official account bot so members can ask questions in natural language.",
    },
    p4: {
      name: "OCR Document Recognition",
      blurb: "Recognizes text in scans and images, then routes it automatically into Capture.",
    },
    p10: {
      name: "Vercel Publishing",
      blurb: "Deploys the Showcase site to Vercel's global CDN with HTTPS and preview links.",
    },
    p12: {
      name: "Claude Processing Engine",
      blurb:
        "Uses Claude to structure raw materials, correct issues, and suggest bidirectional links.",
    },
    p6: {
      name: "Telegram Bot",
      blurb: "Shares the LINE plugin foundation and serves Q&A in Telegram groups and channels.",
    },
    p11: {
      name: "Cloudflare R2",
      blurb:
        "Object storage with zero egress fees, suited for long-term archives of large audio and video originals.",
    },
  },
  bot: {
    b1: {
      name: "Cha-Yu · Member Advisor",
      desc: "A paid-member Q&A bot based on the full Wiki. It can recommend teas and brewing plans.",
    },
    b2: {
      name: "Internal Draft Assistant",
      desc: "Private to you, used to test answer quality after new entries enter the Wiki.",
    },
  },
  deploy: {
    v48: { when: "2 minutes ago", commit: "Added 5 entries including “Rock tea” and “Roasting”" },
    v47: { when: "yesterday 21:14", commit: "Improved homepage AI search copy" },
    v46: { when: "3 days ago", commit: "Fixed contrast in dark mode" },
    v45: { when: "last week", commit: "Migrated to Cloudflare Pages (build failed)" },
  },
};

const JA: Override = {
  lint: {
    l1: {
      title: "孤立項目",
      detail:
        "「紫砂壺の手入れ」「水出し」に双方向リンクがありません。関連する茶器/淹れ方の項目への関連付けを推奨。",
    },
    l2: {
      title: "内容が薄い",
      detail: "「焙煎」は640文字で引用も単一。火抜き周期や火の強さの追記を推奨。",
    },
    l3: {
      title: "重複の疑い",
      detail:
        "「茶ポリフェノール」と「殺青」の酸化に関する記述が大きく重複。統合または相互参照を。",
    },
    l4: {
      title: "更新が古い",
      detail: "「水出し」は1か月未更新で、会員からの問い合わせは増加中。",
    },
  },
  plugin: {
    p1: {
      name: "Whisper 文字起こしエンジン",
      blurb:
        "アップロードされた音声/動画をタイムスタンプ付きテキストへ自動文字起こし。中英混在や方言にも対応。",
    },
    p2: {
      name: "ローカルベクトルDB",
      blurb:
        "Wiki 項目をローカルでベクトル化し、オフライン検索できます。データは端末外に出ません。",
    },
    p3: {
      name: "LINE ボット",
      blurb: "Wiki を LINE 公式アカウントのボットとして公開し、会員が自然言語で質問できます。",
    },
    p4: {
      name: "OCR 文書認識",
      blurb: "スキャンや画像内の文字を認識し、自動で受信箱へ分類します。",
    },
    p10: {
      name: "Vercel 公開",
      blurb:
        "ショーケースサイトを Vercel のグローバル CDN にデプロイし、HTTPS とプレビューリンクを提供します。",
    },
    p12: {
      name: "Claude 加工エンジン",
      blurb: "Claude で原始素材を構造化し、誤りを直し、双方向リンク候補を生成します。",
    },
    p6: {
      name: "Telegram ボット",
      blurb: "LINE プラグインと同じ基盤で、Telegram のグループやチャンネルにQ&Aを提供します。",
    },
    p11: {
      name: "Cloudflare R2",
      blurb:
        "出口料金ゼロのオブジェクトストレージ。大容量の音声/動画素材の長期保管に適しています。",
    },
  },
  bot: {
    b1: {
      name: "茶語 · 会員アドバイザー",
      desc: "有料会員向けのQ&Aボット。完全な Wiki をもとに茶品や淹れ方を提案できます。",
    },
    b2: {
      name: "内部下書きアシスタント",
      desc: "自分だけが見られる、新規項目登録後の回答品質テスト用ボットです。",
    },
  },
  deploy: {
    v48: { when: "2分前", commit: "「岩茶」「焙煎」など5項目を追加" },
    v47: { when: "昨日 21:14", commit: "トップページの AI 検索文言を改善" },
    v46: { when: "3日前", commit: "ダークモードのコントラストを修正" },
    v45: { when: "先週", commit: "Cloudflare Pages に移行（ビルド失敗）" },
  },
};

const OVERRIDES: Partial<Record<Lang, Override>> = { en: EN, ja: JA };

// L('lint', finding, 'title', lang) → localized value, falling back to base[field].
export function L<T extends { id?: string; ver?: string }>(
  kind: string,
  obj: T,
  field: keyof T & string,
  lang: Lang
): string {
  const key = obj.id ?? obj.ver;
  const hit = key ? OVERRIDES[lang]?.[kind]?.[key]?.[field] : undefined;
  return hit ?? String(obj[field]);
}
