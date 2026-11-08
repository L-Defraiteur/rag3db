import { useEffect, useRef, useState } from "react";
import { Check, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { lineDiff } from "@/lib/diff";
import { GraphDiffSection, type GraphDiff } from "@/lib/graph";
import { onFlux, shortName, type FluxEvent, type HostState } from "@/lib/host";

// **Les propositions** (page 08, vue 7 ; page 08 des déclarations) : une
// proposition est un état de l'hôte — la liste se relit à chaque changement
// (sujet `declarations` du flux), le détail montre la différence de chaque
// fichier, le diff de chaque graphe touché sur son dessin, l'aperçu d'un
// composant sans l'appliquer, et Appliquer / Rejeter. Repères gardés :
// `#propositions tr[data-id][data-etat]`, `.proposition[data-etat]`,
// `#proposition-etat`, `#appliquer`, `#rejeter`, `#pourquoi`, `#proposition-note`,
// `.apercu-propose`, `section.graphe-propose[data-cible] g.etape.change[data-nom]`.
type Prop = { id: number; raison: string; auteur: string; etat: string; refus?: string | null; commit?: string | null; pourquoi?: string; fichiers: { path: string; neuf?: boolean; avant?: string | null; apres: string }[] };
const stateOf = (p: Prop) => (p.etat === "appliquee" ? `appliquée (${p.commit ? p.commit.slice(0, 9) : "rien à écrire"})` : p.etat === "rejetee" ? `rejetée : ${p.pourquoi}` : p.refus ? "refusée à la validation" : "proposée");

export function Diff({ before, after }: { before: string; after: string }) {
  const d = lineDiff(before, after);
  if (typeof d === "string") return <div className="diff font-mono text-xs"><div>{d}</div></div>;
  return <div className="diff font-mono text-xs leading-5 overflow-x-auto">{d.map((l, k) => <div key={k} className={`${l.kind} ${l.kind === "plus" ? "bg-green-500/15" : "bg-red-500/15"} whitespace-pre px-2`}>{l.text}</div>)}</div>;
}

export function Propositions({ id, rest, state }: { id: string; rest: string; state: HostState }) {
  if (rest) return <Proposition id={id} pid={Number(rest)} state={state} />;
  return <List id={id} state={state} />;
}

function List({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [list, setList] = useState<Prop[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = () =>
    fetch(`/ide/propositions?${new URLSearchParams({ backend: id })}`)
      .then((r) => r.json())
      .then((r) => (r.ok ? setList([...(r.result as Prop[])].reverse()) : setError(r.error)))
      .catch((e: Error) => setError(e.message));
  useEffect(() => {
    load();
  }, [id]);
  // Une proposition a changé : la liste se relit.
  useEffect(() => onFlux((d: FluxEvent) => { if (backend && d.sujet === "declarations" && d.backend === backend.manifest) load(); }), [backend?.manifest]);
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3"><a href="/ide/" className="hover:underline">backends</a> · {shortName(backend.manifest)} · propositions</p>
      <h1 className="text-xl font-semibold mb-1">Propositions</h1>
      <p className="note text-sm text-muted-foreground mb-3">Ce que l'agent (ou quelqu'un) propose de déclarer : validé sans être écrit, appliqué ou rejeté ici.</p>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {list ? (
        list.length ? (
          <table className="runs w-full text-sm" id="propositions">
            <thead className="text-left text-xs text-muted-foreground"><tr>{["", "raison", "auteur", "fichiers", "état"].map((t, k) => <th key={k} className="py-1 pr-3 font-medium">{t}</th>)}</tr></thead>
            <tbody>
              {list.map((p) => (
                <tr key={p.id} data-id={p.id} data-etat={p.refus && p.etat === "proposee" ? "refusee" : p.etat} className="border-t">
                  <td className="py-1 pr-3"><a href={`/ide/code/${id}/propositions/${p.id}`} className="hover:underline">#{p.id}</a></td>
                  <td className="py-1 pr-3">{p.raison}</td>
                  <td className="py-1 pr-3 text-xs">{p.auteur}</td>
                  <td className="py-1 pr-3 text-xs font-mono">{p.fichiers.map((f) => f.path + (f.neuf ? " (neuf)" : "")).join(", ")}</td>
                  <td className="py-1 text-xs">{stateOf(p)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="note text-sm text-muted-foreground">Aucune proposition pour ce backend.</p>
        )
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}

function Proposition({ id, pid, state }: { id: string; pid: number; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [r, setR] = useState<{ proposition: Prop; graphes: GraphDiff[]; composants: string[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const why = useRef<HTMLInputElement>(null);
  const load = () =>
    fetch(`/ide/propositions?${new URLSearchParams({ backend: id, id: String(pid) })}`)
      .then((x) => x.json())
      .then((x) => (x.ok ? setR(x.result) : setError(x.error)))
      .catch((e: Error) => setError(e.message));
  useEffect(() => {
    load();
  }, [id, pid]);
  const act = async (route: "appliquer" | "rejeter", body: Record<string, string>) => {
    const x = await (await fetch(`/ide/propositions/${route}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, id: pid, ...body }) })).json();
    if (!x.ok) setNote(x.error);
    else await load();
  };
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const p = r?.proposition;
  const open = p?.etat === "proposee";
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3"><a href="/ide/" className="hover:underline">backends</a> · <a href={`/ide/code/${id}/propositions/`} className="hover:underline">{shortName(backend.manifest)}</a> · proposition #{pid}</p>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {r && p ? (
        <div className="proposition grid gap-3" data-etat={p.etat}>
          <h1 className="text-xl font-semibold">Proposition #{p.id}</h1>
          <p className="note text-sm text-muted-foreground">{p.raison} — par {p.auteur} — <strong id="proposition-etat" className="text-foreground">{stateOf(p)}</strong></p>
          {p.refus ? <pre className="refus text-destructive text-xs whitespace-pre-wrap">{p.refus}</pre> : null}
          {open ? (
            <div className="actions flex flex-wrap items-center gap-2">
              {p.refus ? null : <Button id="appliquer" size="sm" onClick={() => act("appliquer", {})}><Check className="size-3.5" /> Appliquer</Button>}
              <input id="pourquoi" ref={why} placeholder="pourquoi rejeter" aria-label="pourquoi rejeter" className="rounded-md border bg-input/30 px-2 py-1.5 text-sm" />
              <Button id="rejeter" size="sm" variant="outline" onClick={() => act("rejeter", { why: why.current?.value || "rejetée dans l'IDE" })}><X className="size-3.5" /> Rejeter</Button>
              <span className="note text-xs text-muted-foreground" id="proposition-note">{note}</span>
            </div>
          ) : null}
          {r.composants.length ? (
            <p className="text-sm">Aperçu, sans appliquer : {r.composants.map((c) => <a key={c} href={`/ide/apercu/${id}/${encodeURIComponent(c)}?proposition=${pid}`} target="_blank" className="apercu-propose underline mr-2" rel="noreferrer">{c}</a>)}</p>
          ) : null}
          {r.graphes.map((g, k) => <GraphDiffSection key={k} g={g} />)}
          {p.fichiers.map((f) => (
            <section key={f.path} className="rounded-lg border overflow-hidden">
              <h2 className="text-sm font-semibold px-3 py-2 border-b bg-muted/40">{f.path}{f.avant == null ? " (neuf)" : ""}</h2>
              <Diff before={f.avant ?? ""} after={f.apres} />
            </section>
          ))}
        </div>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
