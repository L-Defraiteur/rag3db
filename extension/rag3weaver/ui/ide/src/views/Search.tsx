import { useEffect, useMemo, useState } from "react";
import { Search as SearchIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { callFull, shortName, unwrap, type HostState } from "@/lib/host";
import { fileLink, symbolLink } from "@/views/File";

// **La recherche** (page 08, vue 4) : le JSON de `search_code` tel que la session
// recherche le rend — `result[]` (uuid, score, entity, `data` en snake_case, `chunk`,
// `signal`, `signals[{label, score, rank}]`, bientôt `highlights[{start, end,
// source}]` en unités UTF-16 dans `chunk.text`), `metadata["render.meta"]` (les
// signaux actifs, les comptes, `warnings` dont la ligne « index : » en tête),
// `freshness`. Les repères DOM sont ceux de l'ancienne page : `#vue .resultat`,
// `.resultat a`, `.score`, `.signaux`, `#recherche-etat`, `.avertissements`,
// `#reclasser` ; les surlignages viendront en `mark.surligne[data-source]`.

type Signal = { label: string; score: number; rank: number };
type Highlight = { start: number; end: number; source: string };
type Hit = {
  uuid?: string;
  score?: number;
  entity?: string;
  signal?: string;
  signals?: Signal[];
  highlights?: Highlight[];
  data?: { file_path?: string; name?: string; start_line?: number; end_line?: number; scope_type?: string; signature?: string };
  chunk?: { text?: string; startLine?: number; endLine?: number };
};
type Meta = { signals?: string[]; warnings?: string[]; bm25Count?: number; vectorCount?: number; sparseCount?: number; fusedCount?: number; rerankedCount?: number; partial?: boolean; consistency?: string; searchTimeMs?: number };
type Freshness = { lag_files?: number; dirty?: boolean };

/// La ligne de tête : l'état de l'index tel que la méta le dit, sinon « à jour », la
/// fraîcheur, et ce que la recherche a compté.
function statusLine(indexState: string | undefined, f: Freshness | undefined, meta: Meta, n: number): string {
  const parts = [indexState ?? "index : à jour"];
  if (f?.lag_files) parts.push(`${f.lag_files} fichier(s) en retard sur l'arbre`);
  if (f?.dirty) parts.push("arbre modifié depuis l'index");
  const counts: string[] = [];
  if (meta.bm25Count != null) counts.push(`mots ${meta.bm25Count}`);
  if (meta.vectorCount != null) counts.push(`sens ${meta.vectorCount}`);
  if (meta.sparseCount) counts.push(`creux ${meta.sparseCount}`);
  if (meta.rerankedCount) counts.push(`relus ${meta.rerankedCount}`);
  parts.push(`${n} résultat(s)${counts.length ? ` (${counts.join(", ")})` : ""}`);
  if (meta.partial) parts.push("partiel");
  return parts.join(" · ");
}

/// L'extrait, avec ses surlignages (unités UTF-16, convertis côté hôte), bornés à
/// 600 caractères comme avant.
function Excerpt({ text, highlights }: { text: string; highlights?: Highlight[] }) {
  const cut = text.length > 600;
  const shown = cut ? text.slice(0, 600) : text;
  const marks = (highlights ?? []).filter((h) => h.start < shown.length && h.end > h.start).sort((a, b) => a.start - b.start);
  if (!marks.length) return <pre className="whitespace-pre-wrap break-words text-xs mt-1 text-foreground/90">{shown}{cut ? "…" : ""}</pre>;
  const parts: React.ReactNode[] = [];
  let last = 0;
  for (const h of marks) {
    const start = Math.max(h.start, last);
    const end = Math.min(h.end, shown.length);
    if (start > last) parts.push(shown.slice(last, start));
    parts.push(<mark key={`${start}-${end}`} className="surligne" data-source={h.source}>{shown.slice(start, end)}</mark>);
    last = end;
  }
  if (last < shown.length) parts.push(shown.slice(last));
  return <pre className="whitespace-pre-wrap break-words text-xs mt-1 text-foreground/90">{parts}{cut ? "…" : ""}</pre>;
}

export function Search({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const params = useMemo(() => new URLSearchParams(location.search), []);
  const q = params.get("q") ?? "";
  // La relecture se décide question par question : le formulaire repart sans `reclasser`.
  const rerank = params.get("reclasser") === "1";
  const [response, setResponse] = useState<Record<string, unknown> | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!backend || !q) return;
    const options: Record<string, unknown> = { limit: 30 };
    if (rerank) options.rerank = { candidates: 20 };
    callFull(backend.manifest, "search_code", { query: q, options })
      .then((r) => {
        const { value, status } = unwrap(r.result);
        setResponse({ ...r, result: value });
        setStatus(status ?? null);
      })
      .catch((e: Error) => setError(e.message));
  }, [backend?.manifest, q, rerank]);

  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const hits: Hit[] = Array.isArray(response?.result) ? (response!.result as Hit[]) : [];
  // La méta porte le nom du nœud qui la rend dans le graphe (`render`, port `meta`).
  const meta = ((response?.metadata as Record<string, unknown> | undefined)?.["render.meta"] as Meta | undefined) ?? {};
  const warnings = meta.warnings ?? [];
  const indexState = warnings.find((w) => w.startsWith("index : "));
  const others = warnings.filter((w) => !w.startsWith("index : "));
  const hasHighlights = hits.some((h) => h.highlights?.length);

  const toggleRerank = (checked: boolean) => {
    const p = new URLSearchParams(location.search);
    if (checked) p.set("reclasser", "1");
    else p.delete("reclasser");
    location.search = p.toString();
  };

  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href={`/ide/code/${id}/arbre/`} className="hover:underline">{shortName(backend.manifest)}</a> · recherche
      </p>
      <form
        id="chercher"
        className="flex gap-2 mb-4"
        onSubmit={(e) => {
          e.preventDefault();
          const v = new FormData(e.currentTarget).get("q");
          if (v) location.href = `/ide/code/${id}/recherche?q=${encodeURIComponent(String(v))}`;
        }}
      >
        <input name="q" type="search" defaultValue={q} placeholder="chercher dans le code" aria-label="chercher dans le code" className="flex-1 rounded-md border bg-input/30 px-3 py-1.5 text-sm" />
        <Button type="submit" size="sm" variant="outline">
          <SearchIcon className="size-3.5" /> Chercher
        </Button>
      </form>
      <h1 className="text-xl font-semibold mb-2">« {q} »</h1>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {status ? <p className="note etat text-xs text-muted-foreground">{status}</p> : null}
      {response ? (
        <>
          <p id="recherche-etat" className="note text-sm text-muted-foreground">{statusLine(indexState, response.freshness as Freshness | undefined, meta, hits.length)}</p>
          <p className="note text-sm text-muted-foreground mb-4">
            <label className="inline-flex items-center gap-2">
              <input type="checkbox" id="reclasser" defaultChecked={rerank} onChange={(e) => toggleRerank(e.target.checked)} /> reclasser : un relecteur relit les 20 premiers (plus lent ; pour une question en phrases)
            </label>
          </p>
          <div className="grid gap-3">
            {hits.map((x, k) => {
              const d = x.data ?? {};
              const excerpt = x.chunk?.text ?? d.signature ?? "";
              return (
                <div key={x.uuid ?? k} className="resultat rounded-lg border bg-card px-3 py-2">
                  <div className="flex flex-wrap items-baseline gap-x-2 text-sm">
                    <a href={fileLink(id, d.file_path ?? "", d.start_line)} className="font-mono hover:underline">{d.file_path ?? "?"}:{d.start_line ?? ""}</a>
                    {d.name ? <a href={symbolLink(id, d.name)} className="hover:underline">{d.name}</a> : null}
                    <span className="meta note text-xs text-muted-foreground">{d.scope_type ?? x.entity ?? ""} · {x.signal ?? ""}</span>
                    <span className="score text-xs" title="le score de l'ordre rendu">★ {Number(x.score ?? 0).toFixed(3)}</span>
                  </div>
                  {(x.signals ?? []).length ? <div className="signaux note text-xs text-muted-foreground">{x.signals!.map((s) => `${s.label} n°${s.rank} (${Number(s.score).toFixed(3)})`).join(" · ")}</div> : null}
                  {excerpt ? <Excerpt text={excerpt} highlights={x.highlights} /> : null}
                </div>
              );
            })}
          </div>
          {hasHighlights ? (
            <p className="legende text-xs text-muted-foreground mt-3">
              surlignages : <mark className="surligne" data-source="lucivy">au caractère (lucivy)</mark> · <mark className="surligne" data-source="sparse">la fenêtre la plus proche du sens (creux)</mark> · <mark className="surligne" data-source="query_words">les mots de la requête</mark>
            </p>
          ) : null}
          {others.length ? (
            <ul className="avertissements mt-4 text-sm text-amber-700 dark:text-amber-400">
              {others.map((w, k) => <li key={k}>⚠ {w}</li>)}
            </ul>
          ) : null}
        </>
      ) : error ? null : q ? (
        <p className="note text-muted-foreground">Recherche…</p>
      ) : (
        <p className="note text-muted-foreground">Une requête, par mots ou par sens.</p>
      )}
    </>
  );
}
