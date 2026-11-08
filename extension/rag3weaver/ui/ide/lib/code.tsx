import type { ReactNode } from "react";
import hljs from "highlight.js/lib/core";
import rust from "highlight.js/lib/languages/rust";
import cpp from "highlight.js/lib/languages/cpp";
import typescript from "highlight.js/lib/languages/typescript";
import javascript from "highlight.js/lib/languages/javascript";
import python from "highlight.js/lib/languages/python";
import bash from "highlight.js/lib/languages/bash";
import json from "highlight.js/lib/languages/json";
import yaml from "highlight.js/lib/languages/yaml";
import sql from "highlight.js/lib/languages/sql";
import xml from "highlight.js/lib/languages/xml";
import markdown from "highlight.js/lib/languages/markdown";
import { fileLink } from "@/views/File";

// **Le code dans le chat** (tickets du 7 novembre 2026, « le code dans le chat n'est
// ni coloré ni relié au dépôt ») : la coloration A, dans la page, par highlight.js
// (le cœur et les langues du dépôt), avec un thème clair et sombre qui suit l'IDE
// (les classes `hljs-*` habillées dans app.css). C'est le repli de la coloration B
// (les spans tree-sitter rendus par l'hôte, lot suivant), qui le gardera pour
// json, bash, mermaid et quand l'hôte ne répond pas. Un chemin cité entre accents
// graves (`src/x.rs:12`) mène à la vue du fichier, à sa ligne.

for (const [nom, langue] of Object.entries({ rust, cpp, typescript, javascript, python, bash, json, yaml, sql, xml, markdown })) {
  hljs.registerLanguage(nom, langue);
}
hljs.registerAliases(["rs"], { languageName: "rust" });
hljs.registerAliases(["c++", "cc", "h", "hpp", "c"], { languageName: "cpp" });
hljs.registerAliases(["ts", "tsx"], { languageName: "typescript" });
hljs.registerAliases(["js", "jsx", "mjs"], { languageName: "javascript" });
hljs.registerAliases(["py"], { languageName: "python" });
hljs.registerAliases(["sh", "shell", "zsh", "console"], { languageName: "bash" });
hljs.registerAliases(["html", "svg"], { languageName: "xml" });

const echapper = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/// Un bloc de code en HTML coloré : la langue déclarée si on la connaît, sinon le
/// texte tel quel (échappé), jamais une langue devinée.
// **La coloration B** (page 12 de codeparsers) : l'hôte colore par les grammaires
// tree-sitter (`POST /ide/highlight`, spans aux décalages UTF-16 de la page) ; un
// bloc s'affiche d'abord sous highlight.js (`data-hl="repli"`), puis, la réponse
// de l'hôte venue et gardée, se rend en classes `hl-*` (`data-hl="hote"`). Une
// langue que les grammaires ne connaissent pas, ou un hôte qui ne répond pas,
// laisse le repli.
type HostSpan = [number, number, string, number, number];
const coloriage = new Map<string, HostSpan[] | null>();
const enCours = new Set<string>();
const abonnes = new Set<() => void>();
/// Être prévenu quand une coloration de l'hôte arrive (pour rendre de nouveau).
export function onHostHighlight(f: () => void): () => void {
  abonnes.add(f);
  return () => abonnes.delete(f);
}
function demanderAHote(cle: string, code: string, langue: string) {
  if (enCours.has(cle) || coloriage.has(cle)) return;
  enCours.add(cle);
  fetch("/ide/highlight", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ text: code, language: langue }) })
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
    .then((v: { spans?: HostSpan[] | null }) => {
      coloriage.set(cle, v.spans ?? null);
      abonnes.forEach((f) => f());
    })
    .catch(() => coloriage.set(cle, null))
    .finally(() => enCours.delete(cle));
}
function rendreSpans(code: string, spans: HostSpan[]): string {
  let html = "";
  let i = 0;
  for (const [, , sorte, a, b] of spans) {
    if (a < i || b > code.length) continue;
    html += echapper(code.slice(i, a)) + `<span class="hl-${echapper(sorte)}">${echapper(code.slice(a, b))}</span>`;
    i = b;
  }
  return html + echapper(code.slice(i));
}

export function highlightBlock(code: string, lang?: string): string {
  // Les lignes numérotées de read_file (« 00012| code ») : le numéro à part, le
  // code coloré d'après l'extension du fichier quand le bloc ne dit pas sa langue.
  const lignes = code.split("\n");
  if (lignes.length && lignes.filter((l) => l).every((l) => /^\d{5}\| /.test(l))) {
    const langue = (lang ?? "").trim().toLowerCase();
    const colore = (t: string) => (langue && hljs.getLanguage(langue) ? hljs.highlight(t, { language: langue, ignoreIllegals: true }).value : echapper(t));
    const corps = lignes
      .filter((l) => l)
      .map((l) => `<span class="ligne"><span class="numero">${Number(l.slice(0, 5))}</span>${colore(l.slice(7))}</span>`)
      .join("\n");
    return `<pre class="lignes-numerotees"><code class="hljs${langue ? ` language-${echapper(langue)}` : ""}">${corps}</code></pre>`;
  }
  const langue = (lang ?? "").trim().toLowerCase();
  if (langue) {
    const cle = `${langue}\u0000${code}`;
    const spans = coloriage.get(cle);
    if (spans) return `<pre><code class="hl language-${echapper(langue)}" data-hl="hote">${rendreSpans(code, spans)}</code></pre>`;
    if (spans === undefined) demanderAHote(cle, code, langue);
  }
  if (langue && hljs.getLanguage(langue)) {
    const html = hljs.highlight(code, { language: langue, ignoreIllegals: true }).value;
    return `<pre><code class="hljs language-${echapper(langue)}" data-hl="repli">${html}</code></pre>`;
  }
  return `<pre><code${langue ? ` class="language-${echapper(langue)}"` : ""}>${echapper(code)}</code></pre>`;
}

/// Un chemin du dépôt cité (`src/x.rs`, `src/x.rs:12`) : le chemin et la ligne.
const CHEMIN = /^([\w.@-]+(?:\/[\w.@-]+)*\.[A-Za-z0-9]+)(?::(\d+))?$/;
export function pathRef(texte: string): { path: string; line?: number } | null {
  const m = CHEMIN.exec(texte.trim());
  if (!m || !m[1].includes("/")) return null;
  return { path: m[1], line: m[2] ? Number(m[2]) : undefined };
}

/// Le code en ligne en HTML : un chemin cité devient un lien vers sa vue.
export function inlineCodeHtml(code: string, backendId: string): string {
  const ref = pathRef(code);
  const inner = `<code>${echapper(code)}</code>`;
  return ref ? `<a class="code-ref" href="${fileLink(backendId, ref.path, ref.line)}" data-path="${echapper(ref.path)}">${inner}</a>` : inner;
}

/// Un texte brut (le message de la personne, la réponse en flux) avec son code en
/// ligne rendu : ce qui est entre accents graves devient du code, un chemin, un lien.
export function withInlineCode(texte: string, backendId: string): ReactNode[] {
  const morceaux = texte.split(/(`[^`\n]+`)/g);
  return morceaux.map((m, k) => {
    if (m.length > 2 && m.startsWith("`") && m.endsWith("`")) {
      const code = m.slice(1, -1);
      const ref = pathRef(code);
      const el = <code key={k}>{code}</code>;
      return ref ? (
        <a key={k} className="code-ref" href={fileLink(backendId, ref.path, ref.line)} data-path={ref.path}>
          {el}
        </a>
      ) : (
        el
      );
    }
    return m;
  });
}
