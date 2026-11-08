import { useEffect, useRef, useState } from "react";
import { History, MousePointerClick, RotateCcw } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { searchLink } from "@/components/SearchBox";
import { shortName, type HostState } from "@/lib/host";
import { fileLink } from "@/views/File";

// **Mes recherches** (8 novembre 2026, décision de Lucie du 11 octobre : « j'aimerais
// éviter qu'on n'ait pas de logs de ce que j'ai cherché ») : les recherches faites
// dans l'IDE et par les agents, lues par `GET /ide/recherches` dans le journal des
// runs, la plus récente d'abord, avec les résultats cliqués. Repères des témoins :
// `#recherches .recherche[data-client][data-run]`, `.clique` sur un résultat cliqué,
// les filtres `#filtre-tout`, `#filtre-ide`, `#filtre-agents`.

type Result = { rang: number; uuid?: string; chemin?: string; ligne?: number; nom?: string };
type Click = { uuid: string; rang?: number; at?: number };
type Search = {
  run: string;
  at?: number;
  client: string;
  humain: boolean;
  outil: string;
  query?: string;
  scope?: string;
  ms?: number;
  ok?: boolean;
  nombre: number;
  resultats: Result[];
  clics: Click[];
};
type Who = "tout" | "ide" | "agents";
const WHO: { key: Who; label: string }[] = [
  { key: "tout", label: "tout" },
  { key: "ide", label: "les miennes" },
  { key: "agents", label: "les agents" },
];

const when = (at?: number) => (at ? new Date(at).toLocaleString("fr-FR", { dateStyle: "short", timeStyle: "short" }) : "");

function Line({ id, s }: { id: string; s: Search }) {
  const clicked = new Set(s.clics.map((c) => c.uuid));
  return (
    <section className="recherche rounded-lg border bg-card px-3 py-2 grid gap-1" data-client={s.humain ? "ide" : s.client} data-run={s.run}>
      <div className="flex flex-wrap items-baseline gap-x-2 text-sm">
        <Badge variant={s.humain ? "default" : "secondary"} className="qui">{s.humain ? "ma recherche" : s.client}</Badge>
        <span className="font-medium">« {s.query ?? ""} »</span>
        {s.scope ? <span className="text-xs text-muted-foreground">dans {s.scope}</span> : null}
        <span className="meta note text-xs text-muted-foreground">
          {s.outil} · {when(s.at)} · {s.nombre} résultat{s.nombre > 1 ? "s" : ""}
          {s.ms != null ? ` · ${s.ms} ms` : ""}
          {s.clics.length ? ` · ${s.clics.length} cliqué${s.clics.length > 1 ? "s" : ""}` : ""}
        </span>
        {s.outil === "search_code" && s.query ? (
          <a href={searchLink(id, s.query, s.scope ?? null)} className="relancer text-xs inline-flex items-center gap-1 hover:underline">
            <RotateCcw className="size-3" /> relancer
          </a>
        ) : null}
      </div>
      {s.resultats.length ? (
        <ol className="text-xs grid gap-0.5">
          {s.resultats.map((r) => {
            const c = r.uuid != null && clicked.has(r.uuid);
            return (
              <li key={`${r.rang}-${r.uuid}`} className={c ? "clique font-medium" : "text-muted-foreground"}>
                <span className="inline-block w-6 text-right mr-1">{r.rang}.</span>
                {r.chemin ? (
                  <a href={fileLink(id, r.chemin, r.ligne)} className="font-mono hover:underline">{r.chemin}{r.ligne ? `:${r.ligne}` : ""}</a>
                ) : (
                  <span className="font-mono">{r.uuid}</span>
                )}
                {r.nom ? <span className="ml-1">{r.nom}</span> : null}
                {c ? <span className="ml-1 inline-flex items-center gap-0.5"><MousePointerClick className="size-3" /> cliqué</span> : null}
              </li>
            );
          })}
        </ol>
      ) : null}
    </section>
  );
}

export function Recherches({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [who, setWho] = useState<Who>(() => {
    const q = new URLSearchParams(location.search).get("qui");
    return q === "ide" || q === "agents" ? q : "tout";
  });
  const [lines, setLines] = useState<Search[] | null>(null);
  const [next, setNext] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // La réponse d'un filtre qui n'est plus le courant (changé pendant le chargement)
  // est ignorée : elle ne s'ajoute pas à la liste du nouveau.
  const current = useRef(who);
  current.current = who;
  const load = (before: string | null) => {
    const asked = who;
    const p = new URLSearchParams({ backend: id, qui: who });
    if (before) p.set("avant", before);
    fetch(`/ide/recherches?${p}`)
      .then((x) => x.json())
      .then((x) => {
        if (current.current !== asked) return;
        if (!x.ok) return setError(x.error);
        setError(null);
        setLines((old) => (before && old ? [...old, ...x.result.recherches] : x.result.recherches));
        setNext(x.result.suite ?? null);
      })
      .catch((e: Error) => setError(e.message));
  };
  useEffect(() => {
    setLines(null);
    load(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, who]);

  if (!backend) return <p className="text-destructive">dépôt inconnu : {id}</p>;
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href="/ide/" className="hover:underline">dépôts</a> · {shortName(backend.manifest)} · mes recherches
      </p>
      <h1 className="text-xl font-semibold mb-1 inline-flex items-center gap-2"><History className="size-5" /> Mes recherches</h1>
      <p className="note text-sm text-muted-foreground mb-2">
        Ce qui a été cherché dans ce dépôt, depuis l'IDE et par les agents, avec les résultats cliqués : le journal des runs le garde.
      </p>
      <div className="flex gap-2 mb-3" role="group" aria-label="qui a cherché">
        {WHO.map((w) => (
          <Button key={w.key} id={`filtre-${w.key}`} size="sm" variant={who === w.key ? "default" : "outline"} aria-pressed={who === w.key} onClick={() => setWho(w.key)}>
            {w.label}
          </Button>
        ))}
      </div>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      <div id="recherches" className="grid gap-2">
        {lines === null && !error ? <p className="note text-muted-foreground">Chargement…</p> : null}
        {lines?.length === 0 ? <p className="note text-sm text-muted-foreground" id="aucune-recherche">aucune recherche encore</p> : null}
        {lines?.map((s) => <Line key={s.run} id={id} s={s} />)}
      </div>
      {next ? (
        <Button variant="outline" size="sm" className="mt-3" id="plus-anciennes" onClick={() => load(next)}>
          plus anciennes
        </Button>
      ) : null}
    </>
  );
}
