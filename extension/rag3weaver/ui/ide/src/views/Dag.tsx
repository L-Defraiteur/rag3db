import { useEffect, useRef, useState } from "react";
import { Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { onFlux, shortName, stopRun, type FluxEvent, type HostState } from "@/lib/host";
import { GraphSvg, type Graph } from "@/lib/graph";

// **Le DAG vivant** (page 08, vue 7 ; C2) : la liste des graphes d'outils, puis le
// graphe d'un outil, d'une réaction ou d'un composant, dessiné par couches ; le
// sujet `dataflow` du flux l'allume nœud par nœud (`g.etape.passe|echec`,
// `text.duree`), `#dag-etat` dit le run, `#arreter-run` l'arrête (C4). Repères de
// l'ancienne page gardés.
export const endOfRun = (ev: Record<string, unknown>) =>
  ev.cancelled_by ? `arrêté ${ev.cancelled_by === "ide" ? "dans l'IDE" : `par ${ev.cancelled_by}`} après ${ev.ms} ms` : `${ev.ok ? "fini" : "échoué"} en ${ev.ms} ms`;

function GraphList({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [tools, setTools] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!backend) return;
    fetch("/ide/describe", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ manifest: backend.manifest }) })
      .then((r) => r.json())
      .then((r) => {
        if (r.ok === false) throw new Error(r.error ?? "describe refusé");
        const d = r.result ?? r;
        setTools(((d.tools ?? []) as { name: string }[]).map((t) => t.name).sort());
      })
      .catch((e: Error) => setError(e.message));
  }, [backend?.manifest]);
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3"><a href="/ide/" className="hover:underline">backends</a> · {shortName(backend.manifest)} · graphes</p>
      <h1 className="text-xl font-semibold mb-1">Graphes des outils</h1>
      <p className="note text-sm text-muted-foreground mb-3">Un outil appelé pendant qu'on regarde son graphe s'y allume, nœud par nœud.</p>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {tools ? <ul className="liste grid gap-0.5 text-sm">{tools.map((n) => <li key={n}><a href={`/ide/code/${id}/dag/tool/${encodeURIComponent(n)}`} className="hover:underline">{n}</a></li>)}</ul> : null}
    </>
  );
}

export function Dag({ id, rest, state }: { id: string; rest: string; state: HostState }) {
  const [genre, ...more] = rest.split("/");
  const name = more.join("/");
  if (!genre || !name) return <GraphList id={id} state={state} />;
  return <LiveDag id={id} genre={genre} name={name} state={state} />;
}

function LiveDag({ id, genre, name, state }: { id: string; genre: string; name: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [g, setG] = useState<Graph | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState("En attente d'un run de ce graphe…");
  const [run, setRun] = useState<string | null>(null);
  const [marks, setMarks] = useState<Map<string, string>>(new Map());
  const [durations, setDurations] = useState<Map<string, string>>(new Map());
  const [stopNote, setStopNote] = useState("");
  const runRef = useRef<string | null>(null);
  runRef.current = run;
  const target = JSON.stringify({ [genre]: name });
  useEffect(() => {
    fetch(`/ide/graphe?${new URLSearchParams({ backend: id, [genre]: name })}`)
      .then((r) => r.json())
      .then((r) => (r.ok ? setG(r.result) : setError(r.error)))
      .catch((e: Error) => setError(e.message));
  }, [id, genre, name]);
  useEffect(
    () =>
      onFlux((d: FluxEvent) => {
        const ev = d.evenement ?? {};
        if (!backend || d.backend !== backend.manifest) return;
        if (ev.kind === "RunStarted" && ev.target && JSON.stringify(ev.target) === target) {
          setRun(String(ev.run));
          setMarks(new Map());
          setDurations(new Map());
          setStatus(`run ${ev.run} : en cours`);
          setStopNote("");
          return;
        }
        if (!runRef.current || ev.run !== runRef.current) return;
        if (ev.kind === "NodeRun") {
          setMarks((m) => new Map(m).set(String(ev.node), ev.ok === false || ev.error ? "echec" : "passe"));
          setDurations((m) => new Map(m).set(String(ev.node), `${ev.ms} ms`));
        } else if (ev.kind === "RunFinished") {
          setStatus(`run ${ev.run} : ${endOfRun(ev)}`);
          setRun(null);
        }
      }),
    [backend?.manifest, target],
  );
  const stop = async () => {
    if (!run) return;
    setStopNote("arrêt demandé…");
    try {
      await stopRun(id, run);
      setStopNote((n) => (n === "arrêt demandé…" ? "" : n));
    } catch (e) {
      setStopNote((e as Error).message);
    }
  };
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const kind = genre === "tool" ? "Outil" : genre === "reaction" ? "Réaction" : "Composant";
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3"><a href="/ide/" className="hover:underline">backends</a> · <a href={`/ide/code/${id}/dag/`} className="hover:underline">{shortName(backend.manifest)}</a> · {genre} {name}</p>
      <h1 className="text-xl font-semibold mb-1">{name}</h1>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {g ? (
        <>
          <p className="note text-sm text-muted-foreground">{kind}, {g.nodes.length} nœud(s), {g.edges.length} arête(s){g.source ? " — " + g.source : ""}.</p>
          <p className="note text-sm" id="dag-etat">{status}</p>
          <p className="barre flex items-center gap-2 my-2">
            <Button id="arreter-run" size="sm" variant="outline" hidden={!run} onClick={stop}><Square className="size-3.5" /> Arrêter</Button>
            <span className="note text-xs text-muted-foreground" id="arret-note">{stopNote}</span>
          </p>
          {g.nodes.length ? <div className="graphe overflow-x-auto"><GraphSvg g={g} classOf={(n) => marks.get(n) ?? ""} durations={durations} /></div> : <p className="note text-sm text-muted-foreground">Ce graphe est vide.</p>}
        </>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
