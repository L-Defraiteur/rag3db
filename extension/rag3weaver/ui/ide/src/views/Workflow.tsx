import { useEffect, useMemo, useState, type ReactNode } from "react";
import { FilePlus2, Plus, Save, GitCompareArrows, Network, SlidersHorizontal, ShieldCheck, Wand2 } from "lucide-react";
import { displayName, type HostState } from "@/lib/host";
import {
  W, HEAD, ROW, colorOf, compatible, declaration, detailOf, isPlumbing, kindOf, layout, portKey, portY, rowsOf, subGraphOf, wired,
  type PortRef, type WfGraph, type WfNode,
} from "@/lib/workflow";

// **The workflow editor, its first lot** (28 November 2026, Lucie: « la bonne présentation de rag3weaver dans l'IDE,
// en commençant par l'éditeur de graphes »): a tool's declared graph read as the mock-ups draw it (Archiviste, Agent):
// the header and its file, the Simple and Edit views, the canvas, then the node, the file and the graph in three
// columns. The ports are those the node schemas declare (`/ide/graphe`); what is not there yet stays in its place,
// disabled, and says what will come (`data-soon`, « bientôt »), never hidden. Markers: `#workflow [data-node]`,
// `[data-port="<node>:<in|out>:<port>"]` with `data-wired` and `data-lit`, `[data-view=simple|edit]`, `[data-soon]`.

/// A piece still to come: in place, disabled, saying « bientôt » and what it will do.
function Soon({ id, icon, label, what }: { id: string; icon?: ReactNode; label: string; what: string }) {
  return (
    <button type="button" disabled aria-disabled="true" data-soon={id} title={what} className="wf-soon">
      {icon}
      <span>{label}</span>
      <span className="wf-soon-badge">bientôt</span>
      <span className="wf-soon-what">{what}</span>
    </button>
  );
}

export function Workflow({ id, rest, state }: { id: string; rest: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [genre, ...more] = rest.split("/");
  const name = more.join("/");
  const [g, setG] = useState<WfGraph | null>(null);
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [edit, setEdit] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [probe, setProbe] = useState<PortRef | null>(null);

  useEffect(() => {
    if (!backend || !genre || !name) return;
    const q = new URLSearchParams({ backend: id, [genre === "reaction" ? "reaction" : "tool"]: name });
    fetch(`/ide/graphe?${q}`)
      .then((r) => r.json())
      .then((r) => {
        if (!r.ok) throw new Error(r.error ?? "graphe refusé");
        setG(r.result as WfGraph);
        setSelected((r.result as WfGraph).nodes[0]?.name ?? null);
      })
      .catch((e: Error) => setError(e.message));
    fetch("/ide/describe", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ manifest: backend.manifest }) })
      .then((r) => r.json())
      .then((r) => {
        const tools = ((r.result ?? r).tools ?? []) as { name: string; description?: string }[];
        setDescription(tools.find((t) => t.name === name)?.description ?? "");
      })
      .catch(() => {});
  }, [backend?.manifest, genre, name]);

  const nodes = useMemo(() => (g ? (edit ? g.nodes : g.nodes.filter((n) => !isPlumbing(n))) : []), [g, edit]);
  if (!backend) return <p className="text-destructive">dépôt inconnu : {id}</p>;
  if (!genre || !name) return <p className="note text-muted-foreground">Choisis un graphe depuis la liste des outils (<a className="underline" href={`/ide/code/${id}/dag`}>graphes</a>).</p>;
  if (error) return <p className="text-destructive" role="alert">{error}</p>;
  if (!g) return <p className="note text-muted-foreground">Chargement du graphe…</p>;

  const edges = g.edges;
  const L = layout(nodes, edges, edit);
  const file = (g.source ?? `${name}.mmd`).split("/").pop()!;
  const sel = g.nodes.find((n) => n.name === selected) ?? null;
  const hidden = g.nodes.length - nodes.length;
  const lit = (p: PortRef) => !!probe && compatible(probe, p);
  const pick = (p: PortRef) => setProbe((q) => (q && portKey(q) === portKey(p) ? null : p));
  const probeText = probe
    ? `${probe.name} : ${probe.type} — ${probe.side === "out" ? "peut aller vers" : "peut venir de"} les ports allumés. Clique-le encore pour l'éteindre.`
    : edit
      ? "Clique un port : les ports qui peuvent s'y brancher s'allument."
      : `${g.nodes.length} nœuds${hidden ? `, dont ${hidden} de plomberie cachés` : ""}. L'édition montre les ports et leurs types.`;

  return (
    <div id="workflow" className="workflow" data-graph={`${g.kind}:${g.name}`}>
      <header className="wf-header">
        <div className="wf-titles">
          <span className="wf-kicker">Workflows · {displayName(backend)} · {g.kind === "reaction" ? "réaction" : "outil"}</span>
          <h1 className="wf-title">{name}</h1>
          {description ? <p className="wf-about">{description}</p> : null}
        </div>
        <div className="wf-actions">
          <span className="wf-file" title={g.source ?? ""}>{file}</span>
          <Soon id="save" icon={<Save size={14} />} label="Enregistrer" what="Les changements faits ici s'écriront dans le .mmd et son layout.json." />
          <Soon id="new-graph" icon={<FilePlus2 size={14} />} label="Nouveau graphe" what="Créer un graphe de zéro, qui écrit son .mmd." />
        </div>
      </header>

      <div className="wf-bar">
        <div className="wf-toggle" role="group" aria-label="La vue">
          <button type="button" data-view="simple" aria-pressed={!edit} onClick={() => { setEdit(false); setProbe(null); }}>Simplifié</button>
          <button type="button" data-view="edit" aria-pressed={edit} onClick={() => setEdit(true)}>Édition</button>
        </div>
        <p className="wf-probe" aria-live="polite">{probeText}</p>
        <Soon id="add-node" icon={<Plus size={14} />} label="Ajouter un nœud" what="Choisir un nœud du registre et le brancher." />
      </div>

      <div className="wf-canvas-wrap">
        <div className="wf-canvas" style={{ width: L.width, height: L.height }}>
          <svg className="wf-wires" width={L.width} height={L.height} aria-hidden="true">
            {L.edges.map((e, k) => {
              const a = L.pos.get(e.from_node)!, z = L.pos.get(e.to_node)!;
              const na = g.nodes.find((n) => n.name === e.from_node)!, nz = g.nodes.find((n) => n.name === e.to_node)!;
              const x1 = a.x + W, x2 = z.x;
              const y1 = edit ? portY(na, edges, a, "out", e.from_port) : a.y + a.h / 2;
              const y2 = edit ? portY(nz, edges, z, "in", e.to_port) : z.y + z.h / 2;
              const out = na.schema?.outputs.find((p) => p.name === e.from_port);
              const touched = probe && ((probe.node === e.from_node && probe.side === "out" && probe.name === e.from_port) || (probe.node === e.to_node && probe.side === "in" && probe.name === e.to_port));
              const mx = (x1 + x2) / 2;
              return (
                <path key={k} d={`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`} fill="none" stroke={colorOf(out?.type ?? "")} strokeWidth={2}
                  opacity={probe ? (touched ? 1 : 0.15) : 0.7} />
              );
            })}
          </svg>
          {nodes.map((n) => {
            const at = L.pos.get(n.name)!;
            return (
              <div key={n.name} className="wf-node" data-node={n.name} data-selected={n.name === selected} data-graph-node={!!subGraphOf(n)}
                style={{ left: at.x, top: at.y, width: W, height: at.h }}>
                <button type="button" className="wf-node-head" onClick={() => setSelected(n.name)} style={{ height: edit ? HEAD : at.h - 4 }}>
                  <span className="wf-kind">{kindOf(n)}{subGraphOf(n) ? <Network size={11} aria-label="nœud-graphe" /> : null}</span>
                  <span className="wf-node-title">{n.name}</span>
                  {!edit && detailOf(n) ? <span className="wf-node-detail">{detailOf(n)}</span> : null}
                </button>
                {edit
                  ? rowsOf(n, edges).map(({ port, row, value }) => {
                      const c = colorOf(port.type);
                      const top = HEAD + row * ROW;
                      const isWired = wired(port, edges);
                      return (
                        <div key={portKey(port)}>
                          <button type="button" className="wf-port" data-port={portKey(port)} data-side={port.side} data-wired={isWired} data-lit={lit(port)}
                            data-probe={probe ? portKey(probe) === portKey(port) : false} onClick={() => pick(port)}
                            aria-label={`${port.side === "out" ? "sortie" : "entrée"} ${port.name}, type ${port.type}${isWired ? ", branchée" : ", libre"}`}
                            style={{ top, color: c, opacity: probe && !lit(port) && portKey(probe) !== portKey(port) ? 0.3 : 1 }}>
                            {port.name} : {port.type}
                          </button>
                          <span className="wf-dot" style={{ top: top + ROW / 2 - 6, [port.side === "out" ? "right" : "left"]: -7, borderColor: c, background: isWired ? c : undefined }} />
                          {value !== undefined ? <span className="wf-value" style={{ top: top + ROW + 1, borderColor: c }} title="La valeur que le nœud lit sur cette entrée libre">{value}</span> : null}
                        </div>
                      );
                    })
                  : null}
              </div>
            );
          })}
        </div>
      </div>

      <div className="wf-panels">
        <section className="wf-panel" aria-labelledby="wf-node-title">
          <h2 id="wf-node-title">Le nœud : {sel?.name ?? "—"}</h2>
          {sel ? (
            <>
              <p className="wf-muted">{sel.schema?.description || kindOf(sel)}</p>
              <dl className="wf-ports-list">
                <dt>Entrées</dt>
                <dd>{(sel.schema?.inputs ?? []).map((p) => `${p.name} : ${p.type}`).join(", ") || "aucune (point d'entrée)"}</dd>
                <dt>Sorties</dt>
                <dd>{(sel.schema?.outputs ?? []).map((p) => `${p.name} : ${p.type}`).join(", ") || "aucune"}</dd>
              </dl>
              <code className="wf-decl">{declaration(sel)}{sel.line ? `   (ligne ${sel.line})` : ""}</code>
              {subGraphOf(sel) ? (
                <Soon id="open-subgraph" icon={<Network size={14} />} label={`Ouvrir ${subGraphOf(sel)!.split("/").pop()}`} what="Ouvrir le graphe que ce nœud cite, et ses réglages." />
              ) : null}
              <Soon id="edit-values" icon={<SlidersHorizontal size={14} />} label="Modifier les valeurs sur le graphe" what="Un widget sur chaque entrée libre, et le nœud valeur." />
            </>
          ) : null}
          <h3>Réglages · {file}</h3>
          <div className="wf-form">
            {Object.entries(g.inputSchema?.properties ?? {}).map(([k, p]) => (
              <label key={k} className="wf-field" title={p.description ?? ""}>
                <span>{k}{g.inputSchema?.required?.includes(k) ? " *" : ""} <em>{p.type ?? ""}</em></span>
                <input readOnly disabled value={p.default === undefined ? "" : typeof p.default === "string" ? p.default : JSON.stringify(p.default)} />
              </label>
            ))}
          </div>
          <Soon id="save-settings" icon={<Save size={14} />} label="Enregistrer les réglages" what="Les valeurs du formulaire écrites dans l'instance du gabarit, dans le backend." />
        </section>

        <section className="wf-panel" aria-labelledby="wf-diff-title">
          <h2 id="wf-diff-title"><GitCompareArrows size={15} /> La modification · {file}</h2>
          <p className="wf-muted">Aucune modification : ce graphe se lit tel que le fichier le déclare.</p>
          <Soon id="diff" icon={<GitCompareArrows size={14} />} label="Voir le diff du .mmd" what="Chaque changement fait ici, ligne à ligne, avant de l'écrire." />
          <Soon id="types-checked" icon={<ShieldCheck size={14} />} label="Types vérifiés au chargement" what="Le moteur refusera une arête entre deux types qui ne se branchent pas, en la nommant." />
        </section>

        <section className="wf-panel" aria-labelledby="wf-graph-title">
          <h2 id="wf-graph-title">Ce que fait le graphe</h2>
          <p className="wf-muted">{description || "Ce graphe n'a pas de description."}</p>
          {g.result ? <p className="wf-muted">Son résultat : <code>{g.result.node}.{g.result.port}</code></p> : null}
          <Soon id="propose" icon={<Wand2 size={14} />} label="Demander une modification à un agent" what="Un agent propose un changement du graphe ; tu l'acceptes ou non." />
        </section>
      </div>
    </div>
  );
}
