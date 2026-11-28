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
// Le flux partagé : un second script, le `SharedWorker` que les onglets partagent.
await build({
  entryPoints: [join(here, "lib/flux-worker.ts")],
  bundle: true,
  minify: true,
  format: "iife",
  target: "es2022",
  outfile: join(out, "flux-worker.js"),
  legalComments: "none",
  logLevel: "warning",
  absWorkingDir: here,
});
// La feuille : le CLI de tailwind lit src/app.css (le thème shadcn, @import "tailwindcss")
// et balaie src/ et components/ pour les classes employées.
execFileSync(join(here, "node_modules/.bin/tailwindcss"), ["-i", join(here, "src/app.css"), "-o", join(out, "app.css"), "--minify", "--cwd", here], { stdio: "inherit" });
// **The editor's fonts, in the sheet** (28 November 2026): the woff2 of @fontsource (OFL-1.1, pinned), latin subset,
// written as data: URIs after tailwind's output — the host serves them with the sheet, no font is fetched elsewhere.
// The list is the faces the workflow editor uses; the order is fixed, so the build stays deterministic.
const faces = [
  ["Chakra Petch", "chakra-petch", 600], ["Chakra Petch", "chakra-petch", 700],
  ["IBM Plex Sans", "ibm-plex-sans", 400], ["IBM Plex Sans", "ibm-plex-sans", 600],
  ["JetBrains Mono", "jetbrains-mono", 400], ["JetBrains Mono", "jetbrains-mono", 700],
];
const fontCss = faces
  .map(([family, pkgName, weight]) => {
    const woff2 = readFileSync(join(here, `node_modules/@fontsource/${pkgName}/files/${pkgName}-latin-${weight}-normal.woff2`)).toString("base64");
    return `@font-face{font-family:"${family}";font-style:normal;font-weight:${weight};font-display:swap;src:url(data:font/woff2;base64,${woff2}) format("woff2")}`;
  })
  .join("");
writeFileSync(join(out, "app.css"), readFileSync(join(out, "app.css"), "utf8") + fontCss);

const sha256 = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const manifest = {
  sha256: { "app.js": sha256(join(out, "app.js")), "app.css": sha256(join(out, "app.css")), "flux-worker.js": sha256(join(out, "flux-worker.js")) },
  versions,
  node: process.version,
};
writeFileSync(join(out, "bundle.json"), JSON.stringify(manifest, null, 2) + "\n");

if (check) {
  let differ = [];
  for (const f of ["app.js", "app.css", "flux-worker.js"]) {
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
