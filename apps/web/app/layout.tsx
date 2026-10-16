import type { Metadata } from "next";
import type { CSSProperties, ReactNode } from "react";
import "./globals.css";

const siteUrl = "https://thehalfmoon.github.io/Orcel/";
const title = "Orcel — Your computer, your terms";
const description =
  "Local-first, policy-checked computer access for MCP-compatible AI agents.";
const socialDescription =
  "A governed path from MCP-compatible AI agents to your own computer.";

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title,
  description,
  applicationName: "Orcel",
  alternates: {
    canonical: siteUrl
  },
  openGraph: {
    title,
    description: socialDescription,
    type: "website",
    url: siteUrl,
    siteName: "Orcel"
  },
  twitter: {
    card: "summary",
    title,
    description: socialDescription
  }
};

export default function RootLayout({ children }: Readonly<{ children: ReactNode }>) {
  const basePath = process.env.ORCEL_WEB_BASE_PATH ?? "";
  const style = {
    "--orcel-mark-mask": `url("${basePath}/brand/orcel-o-white.svg")`
  } as CSSProperties;

  return (
    <html lang="en">
      <body style={style}>{children}</body>
    </html>
  );
}
