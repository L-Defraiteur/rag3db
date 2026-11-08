import { useEffect, useRef, useState } from "react";
import { Square } from "lucide-react";
import { onFlux, shortName, stopRun, type FluxEvent, type HostState } from "@/lib/host";
import { endOfRun } from "@/views/Dag";

// **Les réactions** (page 08, vue 7) : les réactions déclarées (vers leur DAG
// vivant), l'état des montres et le dernier fichier qu'elles ont vu, les runs
// récents allumés par le flux avec ce que chacun a écrit (cumulé par entité et
// changement, ou par relation). Repères gardés : `#montre-vue`, `#runs-recents
// tr[data-reaction][data-run]`, `td.fin` (`.echec`), `button.arreter`,
// `td.ecrit ul.ecritures li[data-entity][data-change]` / `li[data-relation]`.
type Info = { runs: number; errors: string[]; reactions: { name: string; on: string[]; policy: string; description: string }[]; index?: { lag?: { files?: number }; watches?: Record<string, { paused?: string; scans?: number; lists?: number }> } };
type Write = { key: string; attrs: Record<string, string>; text: string; n?: number; plus?: number; moins?: number; tronque?: boolean };
type RunRow = { run: string; reaction: string; at: string; fin: string; echec: boolean; writes: Write[] };

function writeKey(ev: Record<string, unknown>): string {
  if (ev.kind === "RelationsChanged") return `lien|${ev.relation}`;
  const states = ev.change === "transitioned" && ev.from_state != null ? ` (${ev.from_state} → ${ev.to_state})` : "";
  return `entite|${ev.entity}|${ev.change}${states}`;
}
function cumulate(acc: Write | undefined, ev: Record<string, unknown>): Write {
  const key = writeKey(ev);
  const w: Write = acc ?? { key, attrs: {}, text: "" };
  if (ev.kind === "RelationsChanged") {
    w.plus = (w.plus ?? 0) + ((ev.added as unknown[] | undefined)?.length ?? 0);
    w.moins = (w.moins ?? 0) + ((ev.removed as unknown[] | undefined)?.length ?? 0);
    w.tronque = w.tronque || Boolean(ev.truncated);
    w.attrs = { "data-relation": String(ev.relation) };
    w.text = `${ev.relation} : +${w.plus} −${w.moins}${w.tronque ? " (tronqué : le compte entier est dans le catalogue)" : ""}`;
  } else {
    w.n = (w.n ?? 0) + Number(ev.count ?? (ev.uuids as unknown[] | undefined)?.length ?? 1);
    const states = ev.change === "transitioned" && ev.from_state != null ? ` (${ev.from_state} → ${ev.to_state})` : "";
    w.attrs = { "data-entity": String(ev.entity), "data-change": String(ev.change) };
    w.text = `${ev.entity} : ${w.n} ${ev.change}${states}`;
  }
  return w;
}

export function Reactions({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [info, setInfo] = useState<Info | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [seen, setSeen] = useState("Aucun fichier vu depuis l'ouverture de cette page.");
  const [rows, setRows] = useState<RunRow[]>([]);
  const rowsRef = useRef(rows);
  rowsRef.current = rows;
  useEffect(() => {
    fetch(`/ide/reactions?${new URLSearchParams({ backend: id })}`)
      .then((r) => r.json())
      .then((r) => (r.ok ? setInfo(r.result) : setError(r.error)))
      .catch((e: Error) => setError(e.message));
  }, [id]);
  useEffect(
    () =>
      onFlux((d: FluxEvent) => {
        const ev = d.evenement ?? {};
        if (!backend || d.backend !== backend.manifest) return;
        const at = new Date(Number(d.at_ms ?? Date.now())).toLocaleTimeString();
        if (ev.kind === "FileChanged") setSeen(`Dernier fichier vu par la montre « ${ev.watch} » : ${ev.path} (${ev.change}, ${at}).`);
        else if (ev.kind === "RunStarted" && (ev.target as Record<string, unknown> | undefined)?.reaction) {
          setRows((rs) => [{ run: String(ev.run), reaction: String((ev.target as Record<string, unknown>).reaction), at, fin: "en cours", echec: false, writes: [] }, ...rs].slice(0, 50));
        } else if ((ev.kind === "EntitiesChanged" || ev.kind === "RelationsChanged") && rowsRef.current.some((r) => r.run === ev.run)) {
          setRows((rs) => rs.map((r) => (r.run !== ev.run ? r : { ...r, writes: (() => { const k = writeKey(ev); const i = r.writes.findIndex((w) => w.key === k); const w = cumulate(i >= 0 ? { ...r.writes[i] } : undefined, ev); const ws = [...r.writes]; if (i >= 0) ws[i] = w; else ws.push(w); return ws; })() })));
        } else if (ev.kind === "RunFinished" && rowsRef.current.some((r) => r.run === ev.run)) {
          setRows((rs) => rs.map((r) => (r.run !== ev.run ? r : { ...r, fin: endOfRun(ev), echec: ev.ok === false })));
        }
      }),
    [backend?.manifest],
  );
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const watches = Object.entries(info?.index?.watches ?? {});
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3"><a href="/ide/" className="hover:underline">backends</a> · {shortName(backend.manifest)} · réactions</p>
      <h1 className="text-xl font-semibold mb-1">Réactions</h1>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {info ? (
        <>
          <p className="note text-sm text-muted-foreground">{info.runs} run(s) depuis l'ouverture du backend{info.errors.length ? `, ${info.errors.length} erreur(s)` : ""} ; retard des fichiers : {info.index?.lag?.files ?? 0}.</p>
          {info.reactions.length ? (
            <ul className="liste grid gap-0.5 text-sm mt-2">
              {info.reactions.map((x) => (
                <li key={x.name} className="flex flex-wrap items-baseline gap-2">
                  <a href={`/ide/code/${id}/dag/reaction/${encodeURIComponent(x.name)}`} className="hover:underline">{x.name}</a>
                  <span className="meta text-xs text-muted-foreground">sur {x.on.join(", ")} · {x.policy}</span>
                  <span className="meta text-xs text-muted-foreground">{x.description}</span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="note text-sm text-muted-foreground">Ce backend ne déclare aucune réaction.</p>
          )}
          <h2 className="text-base font-semibold mt-4">Montres</h2>
          {watches.length ? (
            <ul className="liste grid gap-0.5 text-sm">{watches.map(([n, w]) => <li key={n}>{n} <span className="meta text-xs text-muted-foreground">{w.paused ? `en pause : ${w.paused}` : `${w.scans ?? 0} balayages, ${w.lists ?? 0} listes`}</span></li>)}</ul>
          ) : (
            <p className="note text-sm text-muted-foreground">Aucune montre en service.</p>
          )}
          <p className="note text-sm text-muted-foreground" id="montre-vue">{seen}</p>
          <h2 className="text-base font-semibold mt-4">Runs récents</h2>
          <table className="runs w-full text-sm" id="runs-recents">
            <thead className="text-left text-xs text-muted-foreground"><tr><th className="py-1 pr-3 font-medium">début</th><th className="py-1 pr-3 font-medium">réaction</th><th className="py-1 pr-3 font-medium">run</th><th className="py-1 pr-3 font-medium">fin</th><th className="py-1 font-medium">écrit</th></tr></thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.run} data-reaction={r.reaction} data-run={r.run} className="border-t align-top">
                  <td className="py-1 pr-3 text-xs">{r.at}</td>
                  <td className="py-1 pr-3"><a href={`/ide/code/${id}/dag/reaction/${encodeURIComponent(r.reaction)}`} className="hover:underline">{r.reaction}</a></td>
                  <td className="py-1 pr-3 font-mono text-xs">{r.run}</td>
                  <td className={`fin py-1 pr-3 text-xs ${r.echec ? "echec text-destructive" : ""}`}>
                    {r.fin}
                    {r.fin === "en cours" ? <button className="arreter ml-1 inline-flex items-center gap-1 underline" onClick={(e) => { const b = e.currentTarget; b.textContent = "arrêt demandé…"; stopRun(id, r.run).then(() => { if (b.textContent === "arrêt demandé…") b.textContent = ""; }).catch((err: Error) => { b.textContent = err.message; }); }}><Square className="size-3" /> Arrêter</button> : null}
                  </td>
                  <td className="ecrit py-1"><ul className="ecritures text-xs">{r.writes.map((w) => <li key={w.key} {...w.attrs}>{w.text}</li>)}</ul></td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
