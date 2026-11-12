import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const MONITORED_GUIDE_SOURCES = [
  {
    id: "vrchat-first-avatar",
    label: "VRChat Creating Your First Avatar",
    url: "https://creators.vrchat.com/avatars/creating-your-first-avatar/",
  },
  {
    id: "vrchat-safety-trust",
    label: "VRChat Safety and Trust System",
    url: "https://docs.vrchat.com/docs/vrchat-safety-and-trust-system",
  },
  {
    id: "vrchat-plus-faq",
    label: "VRChat Plus FAQ",
    url: "https://hello.vrchat.com/vrchat-plus-faq",
  },
  {
    id: "vrchat-account-upgrade",
    label: "VRChat Platform Account Upgrade Help",
    url: "https://help.vrchat.com/hc/en-us/articles/360062659053-I-want-to-turn-my-platform-account-through-Steam-Meta-Pico-or-Viveport-into-a-VRChat-account",
  },
  {
    id: "booth-payment",
    label: "BOOTH Payment Methods",
    url: "https://booth.pm/payment",
  },
  {
    id: "jinxxy-purchase-agreement",
    label: "Jinxxy Purchase Agreement",
    url: "https://jinxxy.com/purchase-agreement",
  },
  {
    id: "gumroad-buyer-help",
    label: "Gumroad Buyer Help",
    url: "https://gumroad.com/help/article/282-how-do-purchases-work-for-my-customers.html",
  },
  {
    id: "payhip-buyer-help",
    label: "Payhip Buyer Help",
    url: "https://help.payhip.com/article/158-buying-from-payhip",
  },
];

const ENTITY_MAP = {
  "&nbsp;": " ",
  "&amp;": "&",
  "&lt;": "<",
  "&gt;": ">",
  "&quot;": "\"",
  "&#39;": "'",
  "&apos;": "'",
};

/** Converts HTML into normalized plain text for hashing. Page text is NEVER saved to disk. */
export function htmlToNormalizedText(html) {
  return String(html)
    .replace(/<!--[\s\S]*?-->/gu, " ")
    .replace(/<(script|style|noscript|svg|template)\b[^>]*>[\s\S]*?<\/\1>/giu, " ")
    .replace(/<br\s*\/?>/giu, "\n")
    .replace(/<\/(p|div|section|article|li|h[1-6]|tr)>/giu, "\n")
    .replace(/<[^>]+>/gu, " ")
    .replace(/&(nbsp|amp|lt|gt|quot|#39|apos);/giu, (match) => ENTITY_MAP[match.toLowerCase()] ?? match)
    .replace(/[ \t\f\v\r]+/gu, " ")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .join("\n");
}

export function sha256Text(text) {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function looksLikeBotChallenge(status, html) {
  if (status === 401 || status === 403 || status === 429 || status >= 500) return true;
  const snippet = String(html).slice(0, 4096).toLowerCase();
  return snippet.includes("cf-chl-")
    || snippet.includes("challenge-platform")
    || snippet.includes("just a moment...")
    || snippet.includes("attention required! | cloudflare");
}

/**
 * Evaluates one fetched response against its recorded hash entry without storing page text.
 * Automated-fetch refusals (403/429/5xx/network error/challenge) return `unreachable`, NEVER `changed`.
 */
export function evaluateSourceOutcome(previousEntry, fetchResult) {
  if (!fetchResult.ok || fetchResult.status !== 200 || looksLikeBotChallenge(fetchResult.status, fetchResult.body)) {
    return {
      outcome: "unreachable",
      reason: fetchResult.reason ?? `http_${fetchResult.status ?? "error"}`,
      textSha256: previousEntry?.textSha256 ?? null,
    };
  }
  const normalized = htmlToNormalizedText(fetchResult.body);
  if (normalized.length < 32) {
    return {
      outcome: "unreachable",
      reason: "empty_extracted_text",
      textSha256: previousEntry?.textSha256 ?? null,
    };
  }
  const nextHash = sha256Text(normalized);
  if (!previousEntry?.textSha256) {
    return { outcome: "initialized", reason: null, textSha256: nextHash };
  }
  if (previousEntry.textSha256 === nextHash) {
    return { outcome: "unchanged", reason: null, textSha256: nextHash };
  }
  return { outcome: "changed", reason: "hash_mismatch", textSha256: nextHash };
}

export async function checkAllGuideSources({
  manifest,
  fetchPage,
  checkedDate = new Date().toISOString().slice(0, 10),
}) {
  const byId = new Map((manifest?.sources ?? []).map((entry) => [entry.id, entry]));
  const report = [];
  const nextSources = [];

  for (const source of MONITORED_GUIDE_SOURCES) {
    const previous = byId.get(source.id);
    let fetchResult;
    try {
      fetchResult = await fetchPage(source.url);
    } catch (error) {
      fetchResult = {
        ok: false,
        status: 0,
        body: "",
        reason: error instanceof Error ? error.message : "network_error",
      };
    }
    const evaluated = evaluateSourceOutcome(previous, fetchResult);
    report.push({
      id: source.id,
      label: source.label,
      url: source.url,
      outcome: evaluated.outcome,
      reason: evaluated.reason,
      previousSha256: previous?.textSha256 ?? null,
      currentSha256: evaluated.textSha256,
    });
    nextSources.push({
      id: source.id,
      url: source.url,
      textSha256: evaluated.textSha256,
      lastCheckedDate: checkedDate,
      lastOutcome: evaluated.outcome,
    });
  }

  return {
    checkedDate,
    changed: report.filter((item) => item.outcome === "changed"),
    unreachable: report.filter((item) => item.outcome === "unreachable"),
    report,
    nextManifest: {
      schemaVersion: "0.1",
      updatedAt: checkedDate,
      sources: nextSources,
    },
  };
}

async function defaultFetchPage(url) {
  const response = await fetch(url, {
    method: "GET",
    redirect: "follow",
    signal: AbortSignal.timeout(15_000),
    headers: {
      accept: "text/html,application/xhtml+xml",
      "user-agent": "VUA-GuideSourceMonitor/0.1 (+https://ph-r.com/vua/)",
    },
  });
  const body = response.ok ? await response.text() : "";
  return { ok: response.ok, status: response.status, body };
}

async function runCli() {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const defaultManifestPath = path.resolve(scriptDir, "../../../docs/release/guide-source-hashes.json");
  const update = process.argv.includes("--update");
  const failOnChange = process.argv.includes("--fail-on-change");
  const manifest = fs.existsSync(defaultManifestPath)
    ? JSON.parse(fs.readFileSync(defaultManifestPath, "utf8"))
    : { schemaVersion: "0.1", sources: [] };

  const result = await checkAllGuideSources({ manifest, fetchPage: defaultFetchPage });
  if (update) {
    fs.writeFileSync(defaultManifestPath, `${JSON.stringify(result.nextManifest, null, 2)}\n`, "utf8");
  }
  process.stdout.write(`${JSON.stringify({
    checkedDate: result.checkedDate,
    changedCount: result.changed.length,
    unreachableCount: result.unreachable.length,
    report: result.report,
  }, null, 2)}\n`);
  if (failOnChange && result.changed.length > 0) {
    process.exit(2);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  runCli().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  });
}
