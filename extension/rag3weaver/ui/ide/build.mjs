// **Le bâti du bundle de l'IDE** (page 08 du 25 octobre 2026) : esbuild pour le TSX,
// tailwindcss pour la feuille, puis templates/ide/app/{app.js, app.css, bundle.json} —
// commités, embarqués par build.rs, servis par l'hôte. `--check` rebâtit dans un dossier
// temporaire et compare octet pour octet au bundle commité : un écart dit que le bundle
// servi n'est pas celui du code.
import { build } from "esbuild";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const crate = resolve(here, "../..");
const target = join(crate, "templates/ide/app");
const check = process.argv.includes("--check");
const out = check ? mkdtempSync(join(tmpdir(), "rag3weaver-ide-")) : target;
mkdirSync(out, { recursive: true });

const pkg = JSON.parse(readFileSync(join(here, "package.json"), "utf8"));
const versions = Object.fromEntries(
  Object.entries({ ...pkg.dependencies, ...pkg.devDependencies }).map(([n, v]) => [n, v]),
);

// Le script : un seul fichier, ES2022, minifié, sans source map servie. React en
// production ; aucune date ni chemin absolu dans la sortie : le bâti est déterministe.
await build({
  entryPoints: [join(here, "src/main.tsx")],
  bundle: true,
  minify: true,
  format: "esm",
  target: "es2022",
  jsx: "automatic",
  define: { "process.env.NODE_ENV": '"production"' },
  outfile: join(out, "app.js"),
  legalComments: "none",
  logLevel: "warning",
  absWorkingDir: here,
});
// La feuille : le CLI de tailwind lit src/app.css (le thème shadcn, @import "tailwindcss")
// et balaie src/ et components/ pour les classes employées.
execFileSync(join(here, "node_modules/.bin/tailwindcss"), ["-i", join(here, "src/app.css"), "-o", join(out, "app.css"), "--minify", "--cwd", here], { stdio: "inherit" });

const sha256 = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const manifest = {
  sha256: { "app.js": sha256(join(out, "app.js")), "app.css": sha256(join(out, "app.css")) },
  versions,
  node: process.version,
};
writeFileSync(join(out, "bundle.json"), JSON.stringify(manifest, null, 2) + "\n");

if (check) {
  let differ = [];
  for (const f of ["app.js", "app.css"]) {
    const a = join(out, f), b = join(target, f);
    if (!existsSync(b) || sha256(a) !== sha256(b)) differ.push(f);
  }
  const committed = existsSync(join(target, "bundle.json")) ? JSON.parse(readFileSync(join(target, "bundle.json"), "utf8")) : null;
  if (committed && JSON.stringify(committed.versions) !== JSON.stringify(versions)) differ.push("bundle.json (versions)");
  rmSync(out, { recursive: true, force: true });
  if (differ.length) {
    console.error(`ROUGE : le bundle servi n'est pas celui du code (${differ.join(", ")}) : relancer scripts/ide_bundle.sh`);
    process.exit(1);
  }
  console.log("VERT : le bundle commité est celui du code");
} else {
  console.log(`bundle écrit sous ${target} : ${JSON.stringify(manifest.sha256)}`);
}
