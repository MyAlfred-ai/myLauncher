import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { addedLines, checkCatalog, checkLine, checkPath, parseWords } from "./verif-publication.mjs";

// Faux secrets assemblés à l'exécution : ce fichier ne doit pas lui-même en contenir.
const fake = (...parts) => parts.join("");
const ORG = "MyAlfred-ai";

describe("secrets", () => {
  it.each([
    ["token GitHub", fake("ghp", "_", "a1B2".repeat(9))],
    ["token GitHub", fake("github", "_pat_", "x".repeat(50))],
    ["clé Anthropic", fake("sk", "-ant-api03-", "z".repeat(40))],
    ["clé OpenAI", fake("sk", "-proj-", "Q".repeat(40))],
    ["clé AWS", fake("AK", "IA", "ABCDEFGHIJKLMNOP")],
    ["secret OAuth Google", fake("GOC", "SPX-", "k".repeat(28))],
    ["clé privée", fake("-----BEGIN ", "OPENSSH ", "PRIVATE KEY-----")],
    ["clé privée", fake("untrusted ", "comment: rsign encrypted ", "secret key")],
    ["mot de passe dans une URL", fake("postgresql://app:", "Sup3rS3cret", "@db:5432/x")],
    ["mot de passe ou clé en clair", fake('password = "', "hunter2hunter2", '"')],
  ])("%s", (rule, line) => {
    expect(checkLine(`x ${line} y`)).toContain(rule);
  });

  it("ignores placeholders and generated values", () => {
    for (const line of [
      '"placeholder": "sk-ant-…"',
      'ml_dotenv DATABASE_URL "postgresql://app:${pg}@localhost:5432/mytrix"',
      'const ACCOUNT: &str = "github-token";',
      "token: Option<String>,",
    ]) {
      expect(checkLine(line)).toEqual([]);
    }
  });
});

describe("informations internes", () => {
  it("flags internal hosts, emails, private IPs and private repositories", () => {
    expect(checkLine(fake("https://intranet.", "myalfred.ai/x"))).toContain("sous-domaine interne de myalfred.ai");
    expect(checkLine(fake("écrire à jean.dupont@", "myalfred.ai"))).toContain("adresse e-mail");
    expect(checkLine(fake("client@", "acme-industrie.fr"))).toContain("adresse e-mail");
    expect(checkLine(fake("serveur 192.168", ".1.20"))).toContain("adresse IP privée");
    expect(checkLine(fake("https://github.com/", ORG, "/myLLM.git"))).toContain("dépôt MyAlfred-ai non public");
  });

  it("allows public contact details, examples and public repositories", () => {
    for (const line of [
      "https://myalfred.ai/contact/ et https://www.myalfred.ai",
      "contact@myalfred.ai, vous@entreprise.fr, test@example.com",
      "icons/128x128@2x.png",
      "MyAlfred-ai/mia, MyAlfred-ai/my-brief, MyAlfred-ai/myLauncher, ghcr.io/myalfred-ai/mybrief/*",
      "http://127.0.0.1:8080 et version 1.5.0.4",
    ]) {
      expect(checkLine(line)).toEqual([]);
    }
  });

  it("only looks for secrets in lockfiles", () => {
    expect(checkLine(fake('"git+ssh://git@', 'github.com/x/y.git"'), { thirdParty: true })).toEqual([]);
  });

  it("matches the private word list as whole words, case-insensitively", () => {
    const words = parseWords("# commentaire\nAcme Industrie\n\nProjet-X\n");
    expect(words).toHaveLength(2);
    expect(checkLine("Installé chez ACME industrie", { words })).toContain("mot de la liste privée");
    expect(checkLine("le projet-x avance", { words })).toContain("mot de la liste privée");
    expect(checkLine("Acme Industriels", { words })).toEqual([]);
  });
});

describe("fichiers", () => {
  it("refuses secret-looking files", () => {
    expect(checkPath(".env")).toBe("fichier .env");
    expect(checkPath("app/.env.production")).toBe("fichier .env");
    expect(checkPath("src-tauri/mylauncher.key")).toBe("clé ou certificat privé");
    expect(checkPath("backups/base.sql")).toBe("base de données ou sauvegarde");
    expect(checkPath(".env.example")).toBeNull();
    expect(checkPath("src/lib/api.ts")).toBeNull();
  });
});

describe("catalogue", () => {
  const app = (over) => ({
    id: "mia",
    name: "MIA",
    tagline: "t",
    description: "d",
    repo: { owner: "MyAlfred-ai", name: "mia" },
    ...over,
  });

  it("accepts the repository's catalog", () => {
    const catalog = JSON.parse(readFileSync("catalog/apps.json", "utf8"));
    const { problems, publicRepos } = checkCatalog(catalog);
    expect(problems).toEqual([]);
    expect(publicRepos).toEqual(["MyAlfred-ai/mia", "MyAlfred-ai/my-brief"]);
  });

  it("only installs MIA and myBrief from their public repositories", () => {
    expect(checkCatalog({ apps: [app({ id: "mytrix", repo: { owner: "MyAlfred-ai", name: "mytrix" } })] }).problems).toEqual([
      "mytrix : application non autorisée à l'installation",
    ]);
    expect(checkCatalog({ apps: [app({ repo: { owner: ORG, name: "myLLM" } })] }).problems).toEqual([
      `mia : dépôt ${ORG}/myLLM non autorisé`,
    ]);
  });

  it("shows nothing but a teaser for an unreleased application", () => {
    const { problems } = checkCatalog({
      apps: [app({ id: "mybrain", comingSoon: true, features: ["x"], description: "Un wiki" })],
    });
    expect(problems).toEqual([
      "mybrain (Bientôt disponible) : champ « features » interdit",
      "mybrain (Bientôt disponible) : description non vide",
    ]);
  });
});

describe("historique", () => {
  it("keeps only added lines, with their commit and file", () => {
    const diff = [
      "@@commit 0123456789abcdef",
      "diff --git a/x.txt b/x.txt",
      "--- a/x.txt",
      "+++ b/x.txt",
      "@@ -1 +1 @@",
      "-ancienne",
      "+nouvelle",
      "+++ /dev/null",
      "+ignorée",
    ].join("\n");
    expect(addedLines(diff)).toEqual([{ commit: "0123456", path: "x.txt", line: "nouvelle" }]);
  });
});
