// Deploys landing/studio to the Cloudflare Pages project `fframes-studio`
// (https://fframes-studio.pages.dev) from the site root. Pages uploads a whole folder and
// has no .assetsignore, so the pages are rebuilt and copied without the Markdown notes first.
// Needs `wrangler login` or CLOUDFLARE_API_TOKEN; creates the project on the first run.
import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const project = "fframes-studio";
const run = (command, args, options = {}) =>
  execFileSync(command, args, { stdio: "inherit", shell: process.platform === "win32", ...options });

run("node", ["scripts/build-studio-delivery.mjs"]);
const out = mkdtempSync(join(tmpdir(), "studio-pages-"));
try {
  cpSync("landing/studio", out, { recursive: true, filter: (path) => !path.endsWith(".md") });
  // Run wrangler from the copy: the repository's wrangler.jsonc names the Workers site's account.
  const wrangler = (args, options) => run("npx", ["--yes", "wrangler@4", ...args], { cwd: out, ...options });
  if (!wrangler(["pages", "project", "list"], { stdio: "pipe" }).toString().includes(`${project}.pages.dev`))
    // --force keeps it a Pages project (*.pages.dev) instead of delegating to Workers.
    wrangler(["pages", "project", "create", project, "--production-branch", "main", "--force"]);
  wrangler(["pages", "deploy", ".", "--project-name", project, "--branch", "main", "--commit-dirty=true"]);
} finally {
  rmSync(out, { recursive: true, force: true });
}
