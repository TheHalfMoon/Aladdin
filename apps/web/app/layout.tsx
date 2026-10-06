import type { Metadata } from "next";
import type { ReactNode } from "react";
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
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
