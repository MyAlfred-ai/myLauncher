#!/usr/bin/env node
// Vérifie qu'une publication respecte PUBLICATION.md : ni secret, ni donnée client, ni
// information interne de MyAlfred.ai, et un catalogue limité aux applications publiques.
//
//   node scripts/verif-publication.mjs                   fichiers du dépôt
//   node scripts/verif-publication.mjs --depuis <ref>    + lignes ajoutées par <ref>..HEAD
//          [--jusqua <ref>]                              (ou <ref>..<jusqua>)
//   node scripts/verif-publication.mjs --hors-ligne      sans interroger GitHub
//
// MOTS_INTERDITS : liste privée (un mot ou une expression par ligne), jamais écrite dans le dépôt.
// Les journaux de CI sont publics : seuls le fichier, la ligne et la règle sont affichés, jamais
// le texte trouvé.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

/** Seules applications que le catalogue peut proposer à l'installation, et leur dépôt. */
export const PUBLIC_APPS = { mia: "MyAlfred-ai/mia", mybrief: "MyAlfred-ai/my-brief" };
/** Dépôts ou espaces MyAlfred-ai qui peuvent être cités (en minuscules). */
const PUBLIC_REPOS = new Set(["mia", "my-brief", "mybrief", "mylauncher"]);
const PUBLIC_HOSTS = new Set(["myalfred.ai", "www.myalfred.ai"]);
const PUBLIC_EMAILS = new Set(["contact@myalfred.ai"]);
const EXAMPLE_EMAIL_DOMAIN =
  /^(?:entreprise\.fr|exemple\.(?:fr|com)|example\.(?:com|org|net)|(?:users\.)?noreply\.github\.com)$/i;
/** Champs permis à une application « Bientôt disponible » : rien qui la présente. */
const TEASER_KEYS = new Set([
  "id", "name", "comingSoon", "tagline", "description", "icon", "color",
  "repo", "version", "dotenvQuote", "targets",
]);
/** Fichiers venus de registres publics : seuls les secrets y sont cherchés. */
const THIRD_PARTY = /(?:^|\/)(?:package-lock\.json|Cargo\.lock)$/;

const FORBIDDEN_FILES = [
  ["fichier .env", (p) => /(?:^|\/)\.env(?:\.[^/]*)?$/.test(p) && !/\.env\.(?:example|exemple|sample)$/.test(p)],
  ["clé ou certificat privé", (p) => /\.(?:pem|key|p12|pfx|jks|keystore|mobileprovision)$/i.test(p)],
  ["clé SSH", (p) => /(?:^|\/)id_(?:rsa|dsa|ecdsa|ed25519)$/.test(p)],
  ["base de données ou sauvegarde", (p) => /\.(?:sqlite3?|db|sql|dump|bak)$/i.test(p)],
];

// Les motifs sont écrits pour ne pas se reconnaître eux-mêmes dans ce fichier.
const SECRET_RULES = [
  ["token GitHub", /\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})/],
  ["clé Anthropic", /\bsk-ant-[A-Za-z0-9_-]{20,}/],
  ["clé OpenAI", /\bsk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{32,}/],
  ["clé AWS", /\b(?:AKIA|ASIA)[0-9A-Z]{16}\b/],
  ["clé Google", /\bAIza[0-9A-Za-z_-]{35}/],
  ["secret OAuth Google", /\bGOCSPX-[A-Za-z0-9_-]{20,}/],
  ["token Slack", /\bxox[abeprs]-[A-Za-z0-9-]{10,}/],
  ["clé Stripe", /\b[rs]k_live_[A-Za-z0-9]{20,}/],
  ["clé privée", new RegExp("-----BEGIN (?:[A-Z]+ )*" + "PRIVATE KEY-----|untrusted " + "comment:.*secret key")],
  ["jeton JWT", /\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}/],
  ["mot de passe dans une URL", /\b[a-z][a-z0-9+.-]*:\/\/[^\s/:@"'`$<>{}]+:[^\s/@"'`$<>{}]{3,}@/i],
  [
    "mot de passe ou clé en clair",
    /\b(?:password|passwd|pwd|mot_de_passe|secret|api_?key|token)["']?\s*[:=]\s*["'][^"'\s$<>{}…]{12,}["']/i,
  ],
];

const IMAGE_TLD = /^(?:png|jpe?g|gif|svg|webp|ico|icns)$/i;

/** Problèmes d'une ligne de texte : noms des règles enfreintes. */
export function checkLine(line, { thirdParty = false, words = [] } = {}) {
  const found = [];
  for (const [rule, re] of SECRET_RULES) if (re.test(line)) found.push(rule);
  if (thirdParty) return found;

  for (const m of line.matchAll(/\b((?:[a-z0-9-]+\.)+myalfred\.ai)\b/gi)) {
    if (!PUBLIC_HOSTS.has(m[1].toLowerCase())) found.push("sous-domaine interne de myalfred.ai");
  }
  for (const m of line.matchAll(/[A-Za-z0-9._%+-]+@([A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.([A-Za-z]{2,}))\b/g)) {
    const [email, domain, tld] = [m[0].toLowerCase(), m[1], m[2]];
    if (IMAGE_TLD.test(tld)) continue; // icon@2x.png
    if (!PUBLIC_EMAILS.has(email) && !EXAMPLE_EMAIL_DOMAIN.test(domain)) found.push("adresse e-mail");
  }
  if (/\b(?:10\.\d{1,3}|192\.168|172\.(?:1[6-9]|2\d|3[01]))\.\d{1,3}\.\d{1,3}\b/.test(line)) {
    found.push("adresse IP privée");
  }
  for (const m of line.matchAll(/\bmyalfred-ai\/([A-Za-z0-9._-]+)/gi)) {
    const name = m[1].toLowerCase().replace(/\.git$/, "");
    if (!PUBLIC_REPOS.has(name)) found.push("dépôt MyAlfred-ai non public");
  }
  for (const w of words) if (w.test(line)) found.push("mot de la liste privée");
  return [...new Set(found)];
}

/** Liste privée de mots interdits, en expressions insensibles à la casse. */
export function parseWords(text) {
  return (text ?? "")
    .split(/\r?\n/)
    .map((w) => w.trim())
    .filter((w) => w && !w.startsWith("#"))
    .map((w) => new RegExp(`(?<![\\p{L}\\p{N}])${w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?![\\p{L}\\p{N}])`, "iu"));
}

/** Règle enfreinte par le nom d'un fichier, ou null. */
export function checkPath(path) {
  for (const [rule, test] of FORBIDDEN_FILES) if (test(path)) return rule;
  return null;
}

/** Catalogue : applications publiques seulement, et rien de plus qu'une annonce pour les autres. */
export function checkCatalog(catalog) {
  const problems = [];
  const publicRepos = [];
  for (const app of catalog.apps ?? []) {
    const id = app.id ?? "?";
    if (app.comingSoon) {
      for (const k of Object.keys(app)) {
        if (!TEASER_KEYS.has(k)) problems.push(`${id} (Bientôt disponible) : champ « ${k} » interdit`);
      }
      if ((app.description ?? "").trim()) problems.push(`${id} (Bientôt disponible) : description non vide`);
      continue;
    }
    const repo = `${app.repo?.owner}/${app.repo?.name}`;
    const expected = PUBLIC_APPS[id];
    if (!expected) problems.push(`${id} : application non autorisée à l'installation`);
    else if (repo.toLowerCase() !== expected.toLowerCase()) problems.push(`${id} : dépôt ${repo} non autorisé`);
    else publicRepos.push(expected);
  }
  return { problems, publicRepos };
}

/** Lignes ajoutées par les commits d'un intervalle : [{ commit, path, line }]. */
export function addedLines(diff) {
  const out = [];
  let commit = "";
  let path = "";
  for (const l of diff.split("\n")) {
    if (l.startsWith("@@commit ")) commit = l.slice(9, 16);
    else if (l.startsWith("+++ ")) path = l.startsWith("+++ b/") ? l.slice(6) : "";
    else if (l.startsWith("+") && path) out.push({ commit, path, line: l.slice(1) });
  }
  return out;
}

function git(...args) {
  return execFileSync("git", args, { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
}

async function repoIsPublic(repo) {
  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
  const res = await fetch(`https://api.github.com/repos/${repo}`, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "verif-publication",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (res.status === 404) return false;
  if (!res.ok) throw new Error(`GitHub a répondu ${res.status}`);
  return (await res.json()).private === false;
}

async function main(argv) {
  const opt = (name) => {
    const i = argv.indexOf(name);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const since = opt("--depuis");
  const until = opt("--jusqua") ?? "HEAD";
  const offline = argv.includes("--hors-ligne");
  const ci = !!process.env.GITHUB_ACTIONS;
  const words = parseWords(process.env.MOTS_INTERDITS);
  const problems = [];
  const report = (where, rule, file, line) => {
    problems.push(`${where} : ${rule}`);
    if (ci) console.log(`::error${file ? ` file=${file}${line ? `,line=${line}` : ""}` : ""}::${rule}`);
  };

  // 1. Fichiers du dépôt, suivis ou nouveaux (pas ceux ignorés par .gitignore).
  const files = git("ls-files", "-z", "--cached", "--others", "--exclude-standard").split("\0").filter(Boolean);
  let scanned = 0;
  for (const path of files) {
    const rule = checkPath(path);
    if (rule) report(path, rule, path);
    let buf;
    try {
      buf = readFileSync(path);
    } catch {
      continue; // supprimé du disque, pas encore du dépôt
    }
    if (buf.subarray(0, 8000).includes(0)) continue; // binaire
    scanned++;
    const thirdParty = THIRD_PARTY.test(path);
    buf.toString("utf8").split("\n").forEach((line, i) => {
      for (const r of checkLine(line, { thirdParty, words })) report(`${path}:${i + 1}`, r, path, i + 1);
    });
  }

  // 2. Lignes ajoutées par les nouveaux commits : l'historique est public lui aussi.
  let commits = 0;
  if (since) {
    const range = `${since}..${until}`;
    commits = Number(git("rev-list", "--count", "--no-merges", range).trim());
    const diff = git("log", "-p", "--no-merges", "--no-color", "--no-ext-diff", "--format=@@commit %H", range);
    for (const { commit, path, line } of addedLines(diff)) {
      const rule = checkPath(path);
      if (rule) report(`commit ${commit}, ${path}`, rule);
      for (const r of checkLine(line, { thirdParty: THIRD_PARTY.test(path), words })) {
        report(`commit ${commit}, ${path}`, r);
      }
    }
  }

  // 3. Catalogue lu par tous les launchers installés.
  const { problems: catalogProblems, publicRepos } = checkCatalog(
    JSON.parse(readFileSync("catalog/apps.json", "utf8")),
  );
  for (const p of catalogProblems) report("catalog/apps.json", p, "catalog/apps.json");
  if (!offline) {
    for (const repo of publicRepos) {
      try {
        if (!(await repoIsPublic(repo))) report("catalog/apps.json", `${repo} n'est pas un dépôt public`);
      } catch (e) {
        report("catalog/apps.json", `visibilité de ${repo} invérifiable (${e.message})`);
      }
    }
  }

  const unique = [...new Set(problems)];
  if (!words.length) {
    const msg = "liste privée MOTS_INTERDITS absente : noms de clients et projets internes non vérifiés";
    console.log(ci ? `::warning::${msg}` : `Attention : ${msg}`);
  }
  if (unique.length) {
    console.error(`\nPublication refusée (voir PUBLICATION.md), ${unique.length} problème(s) :`);
    for (const p of unique) console.error(`  - ${p}`);
    process.exit(1);
  }
  console.log(
    `Règles de publication respectées : ${scanned} fichiers` +
      (since ? `, ${commits} commit(s) depuis ${since.slice(0, 12)}` : "") +
      (offline ? ", dépôts non interrogés (hors ligne)" : `, dépôts publics vérifiés : ${publicRepos.join(", ")}`) +
      ".",
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2)).catch((e) => {
    console.error(`Vérification impossible : ${e.message}`);
    process.exit(2);
  });
}
