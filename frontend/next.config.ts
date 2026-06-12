import type { NextConfig } from "next";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const isProd = process.env.NODE_ENV === "production";
const internalHost = process.env.TAURI_DEV_HOST || "localhost";
const projectRoot = dirname(fileURLToPath(import.meta.url));

const nextConfig: NextConfig = {
  // Tauri は静的ファイルとして UI を読み込むため、SSR は使わない。
  output: "export",
  images: {
    unoptimized: true,
  },
  turbopack: {
    root: projectRoot,
  },
  // 開発時は Tauri WebView が Next.js 開発サーバーからアセットを読む。
  assetPrefix: isProd ? undefined : `http://${internalHost}:3000`,
};

export default nextConfig;
