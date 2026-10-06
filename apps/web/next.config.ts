import type { NextConfig } from "next";

const basePath = process.env.DESKAL_WEB_BASE_PATH ?? "";

if (basePath !== "" && !/^\/[A-Za-z0-9._-]+$/.test(basePath)) {
  throw new Error("DESKAL_WEB_BASE_PATH must be empty or one absolute path segment");
}

const nextConfig: NextConfig = {
  output: "export",
  basePath,
  poweredByHeader: false,
  reactStrictMode: true
};

export default nextConfig;
