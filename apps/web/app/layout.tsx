import type { Metadata } from "next";
import type { CSSProperties, ReactNode } from "react";
import "./globals.css";

export const metadata: Metadata = {
  title: "Deskal — Your computer, your terms",
  description:
    "Local-first, policy-checked computer access for MCP-compatible AI agents.",
  applicationName: "Deskal",
  openGraph: {
    title: "Deskal — Your computer, your terms",
    description:
      "A governed path from MCP-compatible AI agents to your own computer.",
    type: "website"
  }
};

export default function RootLayout({ children }: Readonly<{ children: ReactNode }>) {
  const basePath = process.env.DESKAL_WEB_BASE_PATH ?? "";
  const style = {
    "--deskal-mark-mask": `url("${basePath}/brand/deskal-mark-mask.svg")`
  } as CSSProperties;

  return (
    <html lang="en">
      <body style={style}>{children}</body>
    </html>
  );
}
