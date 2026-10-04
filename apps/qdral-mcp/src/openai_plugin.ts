import { createHash } from "node:crypto";
import type { DiscoveredTool } from "./discovery.js";
import { LOCAL_ONLY_TOOL_NAMES, REMOTE_TOOL_NAMES, TOOL_CONTRACT, securitySchemesFor } from "./tool_contract.js";

/**
 * SG-000067 OpenAI plugin package.
 *
 * The package is a portable Agent Plugins `plugin.json` plus an `mcp.json`
 * declaring exactly one remote Streamable HTTP MCP server: the owner's
 * existing Qdral relay `/mcp` edge. OpenAI-specific display metadata lives
 * only under `extensions["com.openai"].interface`. The package carries
 * metadata and a public logo; it carries no secret and adds no authority:
 * qdrald, the relay's OAuth checks, device binding, the remote-session
 * lease, workspace trust, and local approval still decide every call.
 *
 * Tool metadata in the review bundle comes from in-process discovery
 * through the authoritative builder on the relay transport, so it is the
 * exact `core` profile a remote client sees.
 */

export const PLUGIN_SCHEMA_URL = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
export const MCP_SCHEMA_URL = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";
export const MAX_LISTING_URL_CHARS = 1024;

export interface OpenAIPluginConfig {
  readonly schema: "qdral-openai-plugin-config/1";
  readonly name: string;
  readonly displayName: string;
  readonly description: string;
  readonly shortDescription: string;
  readonly longDescription: string;
  readonly developerName: string;
  readonly category: string;
  readonly capabilities: readonly string[];
  readonly author: { readonly name: string; readonly url: string };
  readonly homepage: string;
  readonly repository: string;
  readonly license: string;
  readonly keywords: readonly string[];
  readonly websiteURL: string;
  readonly supportURL: string;
  readonly privacyPolicyURL: string;
  readonly termsOfServiceURL: string;
  readonly securityURL: string;
  readonly brandColor: string;
  readonly defaultPrompt: readonly string[];
  readonly mcpServerName: string;
}

const LISTING_URL_FIELDS = ["websiteURL", "supportURL", "privacyPolicyURL", "termsOfServiceURL", "securityURL"] as const;

/** Patterns that must never appear in a package or review artifact. */
export const SECRET_PATTERNS: readonly RegExp[] = [
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/,
  /\bsk-[A-Za-z0-9_-]{16,}/,
  /\bey[A-Za-z0-9_-]{10,}\.ey[A-Za-z0-9_-]{10,}\./,
  /client_secret/i,
  /refresh_token"?\s*[:=]\s*"[^"]+"/i,
  /access_token"?\s*[:=]\s*"[^"]+"/i,
  /"d"\s*:\s*"[A-Za-z0-9_-]{20,}"/,
  /\bgh[pousr]_[A-Za-z0-9]{20,}/,
  /\bAKIA[0-9A-Z]{16}\b/,
  /password\s*[:=]\s*\S+/i
];

export function findSecrets(text: string): string[] {
  return SECRET_PATTERNS.filter((pattern) => pattern.test(text)).map((pattern) => pattern.source);
}

function isHttpsUrl(value: unknown, maxChars = MAX_LISTING_URL_CHARS): boolean {
  if (typeof value !== "string" || value.length === 0 || value.length > maxChars) {
    return false;
  }
  try {
    const url = new URL(value);
    return url.protocol === "https:" && url.username === "" && url.password === "" && url.hash === "";
  } catch {
    return false;
  }
}

/** The relay `/mcp` endpoint: https, no credentials, no query, path `/mcp`. */
export function isMcpEndpointUrl(value: unknown): boolean {
  if (!isHttpsUrl(value)) {
    return false;
  }
  const url = new URL(value as string);
  return url.pathname === "/mcp" && url.search === "" && !/^(localhost|127\.|\[?::1\]?$)/.test(url.hostname);
}

export function validateConfig(config: OpenAIPluginConfig): string[] {
  const problems: string[] = [];
  if (config.schema !== "qdral-openai-plugin-config/1") problems.push("config schema");
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(config.name)) problems.push("name must be kebab-case");
  for (const field of LISTING_URL_FIELDS) {
    if (!isHttpsUrl(config[field])) problems.push(`${field} must be an https URL of at most ${MAX_LISTING_URL_CHARS} characters`);
  }
  for (const field of ["homepage", "repository"] as const) {
    if (!isHttpsUrl(config[field])) problems.push(`${field} must be an https URL`);
  }
  if (!/^#[0-9A-Fa-f]{6}$/.test(config.brandColor)) problems.push("brandColor must be #RRGGBB");
  if (!/^[a-z0-9-]{1,64}$/.test(config.mcpServerName)) problems.push("mcpServerName");
  if (config.description.length === 0 || config.description.length > 1024) problems.push("description length");
  if (config.defaultPrompt.length === 0 || config.defaultPrompt.length > 5) problems.push("defaultPrompt count");
  return problems;
}

export interface PackageFiles {
  readonly [path: string]: string;
}

/**
 * Build the package files. `version` is the Qdral release version; `mcpUrl`
 * is the owner's deployed relay `/mcp` endpoint, supplied at build time and
 * never committed, so no placeholder endpoint is ever published.
 */
export function buildOpenAIPluginPackage(input: {
  readonly config: OpenAIPluginConfig;
  readonly version: string;
  readonly mcpUrl: string;
  readonly logoSvg: string;
  readonly tools: readonly DiscoveredTool[];
  /** Committed review sources copied under `review/`. */
  readonly reviewFiles?: Readonly<Record<string, string>>;
}): PackageFiles {
  const { config, version, mcpUrl } = input;
  const problems = validateConfig(config);
  if (!/^\d+\.\d+\.\d+$/.test(version)) problems.push("version must be semantic");
  if (!isMcpEndpointUrl(mcpUrl)) problems.push("mcpUrl must be a public https relay /mcp endpoint without credentials or query");
  if (problems.length > 0) {
    throw new Error(`OpenAI plugin configuration invalid: ${problems.join("; ")}`);
  }
  const plugin = {
    $schema: PLUGIN_SCHEMA_URL,
    name: config.name,
    version,
    description: config.description,
    author: config.author,
    homepage: config.homepage,
    repository: config.repository,
    license: config.license,
    keywords: config.keywords,
    extensions: {
      "com.openai": {
        interface: {
          displayName: config.displayName,
          shortDescription: config.shortDescription,
          longDescription: config.longDescription,
          developerName: config.developerName,
          category: config.category,
          capabilities: config.capabilities,
          websiteURL: config.websiteURL,
          privacyPolicyURL: config.privacyPolicyURL,
          termsOfServiceURL: config.termsOfServiceURL,
          defaultPrompt: config.defaultPrompt,
          brandColor: config.brandColor,
          composerIcon: "./assets/logo.svg",
          logo: "./assets/logo.svg"
        }
      }
    }
  };
  const mcp = {
    $schema: MCP_SCHEMA_URL,
    mcpServers: {
      [config.mcpServerName]: {
        type: "streamable-http",
        url: mcpUrl
      }
    }
  };
  const toolScan = input.tools.map((tool) => ({
    name: tool.name,
    title: tool.title,
    description: tool.description,
    inputSchema: tool.inputSchema,
    annotations: tool.annotations,
    securitySchemes: (tool._meta?.securitySchemes as unknown) ?? null
  }));
  const files: Record<string, string> = {
    "plugin.json": JSON.stringify(plugin, null, 2) + "\n",
    "mcp.json": JSON.stringify(mcp, null, 2) + "\n",
    "assets/logo.svg": input.logoSvg,
    "review/tool-scan.json": JSON.stringify({ schema: "qdral-openai-tool-scan/1", profile: "core", tools: toolScan }, null, 2) + "\n"
  };
  for (const [name, text] of Object.entries(input.reviewFiles ?? {})) {
    if (!/^[A-Za-z0-9_.-]+$/.test(name) || name === "tool-scan.json") {
      throw new Error(`invalid review file name ${name}`);
    }
    files[`review/${name}`] = text;
  }
  const digests = Object.keys(files)
    .sort()
    .map((path) => `${createHash("sha256").update(files[path] ?? "").digest("hex")}  ${path}`);
  files["SHA256SUMS.txt"] = digests.join("\n") + "\n";
  return files;
}

/**
 * Validate a built package against the format, the tool contract, and the
 * secret rules. Returns a list of problems; an empty list means valid.
 */
export function validateOpenAIPluginPackage(files: PackageFiles): string[] {
  const problems: string[] = [];
  const parse = (path: string): Record<string, unknown> | null => {
    try {
      return JSON.parse(files[path] ?? "") as Record<string, unknown>;
    } catch {
      problems.push(`${path} is missing or not JSON`);
      return null;
    }
  };
  const plugin = parse("plugin.json");
  const mcp = parse("mcp.json");
  const scan = parse("review/tool-scan.json");
  if (plugin !== null) {
    if (plugin.$schema !== PLUGIN_SCHEMA_URL) problems.push("plugin.json $schema");
    for (const field of ["name", "version", "description"]) {
      if (typeof plugin[field] !== "string" || (plugin[field] as string).length === 0) problems.push(`plugin.json ${field}`);
    }
    const allowedTop = new Set(["$schema", "name", "version", "description", "author", "homepage", "repository", "license", "keywords", "extensions"]);
    for (const key of Object.keys(plugin)) {
      if (!allowedTop.has(key)) problems.push(`plugin.json unexpected field ${key}`);
    }
    const extensions = plugin.extensions as Record<string, unknown> | undefined;
    if (extensions === undefined || Object.keys(extensions).join(",") !== "com.openai") {
      problems.push("OpenAI settings must live only under extensions.com.openai");
    }
    const iface = (extensions?.["com.openai"] as Record<string, unknown> | undefined)?.interface as Record<string, unknown> | undefined;
    for (const field of ["websiteURL", "privacyPolicyURL", "termsOfServiceURL"]) {
      if (!isHttpsUrl(iface?.[field])) problems.push(`interface.${field}`);
    }
    for (const field of ["composerIcon", "logo"]) {
      const asset = iface?.[field];
      if (typeof asset !== "string" || !asset.startsWith("./assets/") || asset.includes("..") || files[asset.slice(2)] === undefined) {
        problems.push(`interface.${field} must be a packaged ./assets/ path`);
      }
    }
  }
  if (mcp !== null) {
    if (mcp.$schema !== MCP_SCHEMA_URL) problems.push("mcp.json $schema");
    const servers = Object.values((mcp.mcpServers as Record<string, Record<string, unknown>> | undefined) ?? {});
    if (servers.length !== 1) problems.push("mcp.json must declare exactly one server");
    for (const server of servers) {
      if (server.type !== "streamable-http") problems.push("mcp server type must be streamable-http");
      if (!isMcpEndpointUrl(server.url)) problems.push("mcp server url must be a public https /mcp endpoint");
      for (const key of Object.keys(server)) {
        if (key !== "type" && key !== "url") problems.push(`mcp server carries unexpected field ${key}`);
      }
    }
  }
  if (scan !== null) {
    const tools = (scan.tools as Array<Record<string, unknown>> | undefined) ?? [];
    const names = tools.map((tool) => String(tool.name)).sort();
    if (JSON.stringify(names) !== JSON.stringify([...REMOTE_TOOL_NAMES].sort())) {
      problems.push("tool scan must list exactly the remote core profile");
    }
    for (const tool of tools) {
      const name = String(tool.name);
      if (LOCAL_ONLY_TOOL_NAMES.includes(name)) problems.push(`local-only tool ${name} is exposed`);
      const entry = TOOL_CONTRACT[name];
      if (entry === undefined) {
        problems.push(`tool ${name} has no contract entry`);
        continue;
      }
      const { title, ...hints } = entry.annotations;
      if (JSON.stringify(tool.annotations) !== JSON.stringify({ title, ...hints })) problems.push(`${name} annotations differ from the contract`);
      if (JSON.stringify(tool.securitySchemes) !== JSON.stringify(securitySchemesFor(name))) problems.push(`${name} securitySchemes differ from the contract`);
      if (typeof tool.description !== "string" || tool.description.length === 0) problems.push(`${name} description`);
    }
  }
  for (const [path, text] of Object.entries(files)) {
    if (path.startsWith("/") || path.includes("..")) problems.push(`unsafe package path ${path}`);
    for (const secret of findSecrets(text)) problems.push(`${path} matches secret pattern ${secret}`);
  }
  const sums = files["SHA256SUMS.txt"];
  if (sums !== undefined) {
    for (const line of sums.trim().split("\n")) {
      const [digest, path] = line.split("  ");
      if (path === undefined || createHash("sha256").update(files[path] ?? "").digest("hex") !== digest) problems.push(`checksum mismatch ${path}`);
    }
  } else {
    problems.push("SHA256SUMS.txt missing");
  }
  return problems;
}
