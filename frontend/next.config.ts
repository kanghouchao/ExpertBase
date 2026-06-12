import type { NextConfig } from "next";

const isProd = process.env.NODE_ENV === "production";
const internalHost = process.env.TAURI_DEV_HOST || "localhost";

const nextConfig: NextConfig = {
  // Tauri loads the UI from static files; SSR is not available.
  output: "export",
  images: {
    unoptimized: true,
  },
  // In dev the Tauri webview loads assets from the Next.js dev server.
  assetPrefix: isProd ? undefined : `http://${internalHost}:3000`,
};

export default nextConfig;
