import { useEffect, useMemo, useState, type ReactNode } from "react";
import { FilePlus2, Plus, Save, GitCompareArrows, Network, SlidersHorizontal, ShieldCheck, Wand2 } from "lucide-react";
import { displayName, type HostState } from "@/lib/host";
import { lineDiff } from "@/lib/diff";
import {
  W, HEAD, ROW, MAX_SETTINGS, colorOf, compatible, declaration, detailOf, isPlumbing, kindOf, layout, portKey, portY, rowsOf, settingsOf, settingsRow, subGraphOf, valueText, wired,
  choicesOf, type Draft, type Edit, type ParamDecl, type PortRef, type WfGraph, type WfNode,
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
  // A graph node's cited graph: `?node=core/decide`, a path of graph nodes from the tool's graph down.
  const node = new URLSearchParams(location.search).get("node") ?? "";
  const here = (path: string) => `/ide/code/${id}/workflow/${genre}/${encodeURIComponent(name)}${path ? `?node=${encodeURIComponent(path)}` : ""}`;
  const [g, setG] = useState<WfGraph | null>(null);
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [edit, setEdit] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [probe, setProbe] = useState<PortRef | null>(null);
  // The draft (the third lot, 28 November 2026): edits the host applies to the `.mmd` line by line, nothing written.
  const [edits, setEdits] = useState<Edit[]>([]);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [draftError, setDraftError] = useState<string | null>(null);

  useEffect(() => {
    if (!backend || !genre || !name) return;
    const q = new URLSearchParams({ backend: id, [genre === "reaction" ? "reaction" : "tool"]: name });
    if (node) q.set("node", node);
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
  }, [backend?.manifest, genre, name, node]);

  useEffect(() => {
    if (!edits.length) {
      setDraft(null);
      setDraftError(null);
      return;
    }
    fetch("/ide/graphe/edit", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, [genre === "reaction" ? "reaction" : "tool"]: name, edits }) })
      .then((r) => r.json())
      .then((r) => {
        if (!r.ok) throw new Error(r.error ?? "modification refusée");
        setDraft(r.result as Draft);
        setDraftError(null);
      })
      .catch((e: Error) => setDraftError(e.message));
  }, [edits]);
  const view: WfGraph | null = g && draft && draft.nodes.length ? { ...g, nodes: draft.nodes, edges: draft.edges } : g;
  const nodes = useMemo(() => (view ? (edit ? view.nodes : view.nodes.filter((n) => !isPlumbing(n))) : []), [view, edit]);
  if (!backend) return <p className="text-destructive">dépôt inconnu : {id}</p>;
  if (!genre || !name) return <p className="note text-muted-foreground">Choisis un graphe depuis la liste des outils (<a className="underline" href={`/ide/code/${id}/dag`}>graphes</a>).</p>;
  if (error) return <p className="text-destructive" role="alert">{error}</p>;
  if (!g) return <p className="note text-muted-foreground">Chargement du graphe…</p>;

  const edges = view!.edges;
  const all = view!.nodes;
  // A cited graph is read here; its edits come with the instance (the ticket of 26 November).
  const editable = !node;
  const push = (e: Edit) => setEdits((es) => [...es.filter((x) => !(e.op === "set" && x.op === "set" && x.node === e.node && x.key === e.key)), e]);
  const L = layout(nodes, edges, edit);
  const file = (g.source ?? `${name}.mmd`).split("/").pop()!;
  const sel = all.find((n) => n.name === selected) ?? null;
  const hidden = all.length - nodes.length;
  const lit = (p: PortRef) => !!probe && compatible(probe, p);
  // A port is a target (the ticket of 26 November): an output, then a lit input, makes an edge (or the other way).
  const pick = (p: PortRef) => {
    if (probe && editable && lit(p)) {
      const [o, i] = probe.side === "out" ? [probe, p] : [p, probe];
      push({ op: "connect", from_node: o.node, from_port: o.name, to_node: i.node, to_port: i.name });
      setProbe(null);
      return;
    }
    setProbe((q) => (q && portKey(q) === portKey(p) ? null : p));
  };
  const probeText = probe
    ? `${probe.name} : ${probe.type} — ${probe.side === "out" ? "peut aller vers" : "peut venir de"} les ports allumés. Clique-le encore pour l'éteindre.`
    : edit
      ? "Clique un port : les ports qui peuvent s'y brancher s'allument."
      : `${all.length} nœuds${hidden ? `, dont ${hidden} de plomberie cachés` : ""}. L'édition montre les ports et leurs types.`;

  return (
    <div id="workflow" className="workflow" data-graph={`${g.kind}:${g.name}`}>
      <header className="wf-header">
        <div className="wf-titles">
          <span className="wf-kicker">Workflows · {displayName(backend)} · {g.kind === "reaction" ? "réaction" : g.kind === "subgraph" ? "graphe cité" : "outil"}</span>
          <h1 className="wf-title">{g.kind === "subgraph" ? file.replace(/\.mmd$/, "") : name}</h1>
          {g.path?.length ? (
            <nav className="wf-crumbs" aria-label="D'où vient ce graphe" data-crumbs={g.path.join("/")}>
              <a href={here("")}>{name}</a>
              {g.path.map((step, i) => (
                <span key={i}> › {i === g.path!.length - 1 ? <b>{step}</b> : <a href={here(g.path!.slice(0, i + 1).join("/"))}>{step}</a>}</span>
              ))}
            </nav>
          ) : null}
          {description ? <p className="wf-about" title={description}>{description}</p> : null}
        </div>
        <div className="wf-actions">
          <span className="wf-file" title={g.source ?? ""}>{file}</span>
          <Soon id="save" icon={<Save size={14} />} label="Enregistrer" what="Les changements du brouillon s'écriront dans l'instance du gabarit, ou en nouvelle version du .mmd." />
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
              const na = all.find((n) => n.name === e.from_node)!, nz = all.find((n) => n.name === e.to_node)!;
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
                {edit
                  ? (() => {
                      const s = settingsOf(n);
                      const top = HEAD + settingsRow(n, edges) * ROW + 6;
                      return (
                        <div className="wf-settings" data-settings={n.name} style={{ top }}>
                          {s.slice(0, MAX_SETTINGS).map(({ key, text }) => (
                            <span key={key} className="wf-setting" title={`${key} : ${text}`}><b>{key}</b> {text.startsWith("← ") ? text : `= ${text}`}</span>
                          ))}
                          {s.length > MAX_SETTINGS ? <span className="wf-setting wf-more">+{s.length - MAX_SETTINGS} réglages, dans le panneau du nœud</span> : null}
                        </div>
                      );
                    })()
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
              {settingsOf(sel).length ? (
                <dl className="wf-ports-list" data-node-settings={sel.name}>
                  {settingsOf(sel).map(({ key, text }) => (
                    <div key={key} className="contents"><dt>{key}</dt><dd>{text}</dd></div>
                  ))}
                </dl>
              ) : null}
              <code className="wf-decl">{declaration(sel)}{sel.line ? `   (ligne ${sel.line})` : ""}</code>
              {subGraphOf(sel) ? (
                <a className="wf-open" data-open-subgraph={sel.name} href={here([...(g.path ?? []), sel.name].join("/"))}>
                  <Network size={14} /> Ouvrir {subGraphOf(sel)!.split("/").pop()} et ses réglages
                </a>
              ) : null}
              {editable ? (
                <>
                  <h3>Réglages du nœud</h3>
                  <div className="wf-form" data-node-widgets={sel.name}>
                    {(sel.schema?.parameters ?? []).map((p) => (
                      <Widget key={`${sel.name}.${p.name}`} p={p} value={(sel.config ?? {})[p.name]} onSet={(value) => push({ op: "set", node: sel.name, key: p.name, value })} />
                    ))}
                  </div>
                  <h3>Ses fils</h3>
                  <ul className="wf-edges" data-node-edges={sel.name}>
                    {edges
                      .filter((e) => e.from_node === sel.name || e.to_node === sel.name)
                      .map((e) => (
                        <li key={`${e.from_node}.${e.from_port}>${e.to_node}.${e.to_port}`}>
                          <code>{e.from_node}.{e.from_port} → {e.to_node}.{e.to_port}</code>
                          <button type="button" data-action="disconnect" aria-label={`Débrancher ${e.from_node}.${e.from_port} → ${e.to_node}.${e.to_port}`}
                            onClick={() => push({ op: "disconnect", ...e })}>×</button>
                        </li>
                      ))}
                  </ul>
                </>
              ) : (
                <Soon id="edit-cited" icon={<SlidersHorizontal size={14} />} label="Modifier ce graphe cité" what="Ses valeurs vivront dans l'instance du gabarit, dans le backend." />
              )}
            </>
          ) : null}
          <h3>Réglages · {file}</h3>
          <div className="wf-form">
            {Object.entries(g.inputSchema?.properties ?? {}).map(([k, p]) => (
              <label key={k} className="wf-field" title={p.description ?? ""}>
                <span>{k}{g.inputSchema?.required?.includes(k) ? " *" : ""} <em>{p.type ?? ""}</em></span>
                <input readOnly disabled value={p.default === undefined ? "" : typeof p.default === "string" ? p.default : JSON.stringify(p.default)} />
                {g.passed && k in g.passed ? <small className="wf-passed" data-passed={k}>donné par le nœud : {valueText(g.passed[k])}</small> : null}
              </label>
            ))}
          </div>
          <Soon id="save-settings" icon={<Save size={14} />} label="Enregistrer les réglages" what="Les valeurs du formulaire écrites dans l'instance du gabarit, dans le backend." />
        </section>

        <section className="wf-panel" aria-labelledby="wf-diff-title">
          <h2 id="wf-diff-title"><GitCompareArrows size={15} /> La modification · {file}</h2>
          {draftError ? <p className="wf-refused" role="alert">{draftError}</p> : null}
          {draft ? (
            <>
              <p className={draft.valid ? "wf-muted" : "wf-refused"} data-draft-valid={String(draft.valid)} role={draft.valid ? undefined : "alert"}>
                <ShieldCheck size={13} /> {draft.valid ? "Le graphe modifié se charge : types et fils vérifiés." : `Il ne se chargerait pas : ${draft.error}`}
              </p>
              <pre className="wf-diff" data-diff>
                {(() => {
                  const d = lineDiff(draft.before, draft.after);
                  return typeof d === "string" ? d : d.map((l, i) => <span key={i} className={l.kind === "plus" ? "wf-plus" : "wf-minus"}>{l.kind === "plus" ? "+ " : "- "}{l.text}{"\n"}</span>);
                })()}
              </pre>
              <button type="button" className="wf-reset" data-action="reset" onClick={() => setEdits([])}>Annuler les changements</button>
            </>
          ) : (
            <p className="wf-muted">Aucune modification : ce graphe se lit tel que le fichier le déclare. Branche un port sur un autre, ou change un réglage du nœud.</p>
          )}
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

/// **A setting's widget, by its declared type** (the ticket of 26 November): a toggle for a boolean, a number field for
/// a number, a list for a closed list, a small editor for JSON, a field for a text. A `$param` reads « ← param » until a
/// value is written over it.
function Widget({ p, value, onSet }: { p: ParamDecl; value: unknown; onSet: (v: unknown) => void }) {
  const ref = typeof value === "string" && value.startsWith("$") ? value.slice(1) : null;
  const shown = value === undefined ? p.default : value;
  const choices = choicesOf(p);
  const text = shown === undefined || shown === null ? "" : typeof shown === "string" ? shown : JSON.stringify(shown);
  let control: ReactNode;
  if (p.type === "Bool") {
    control = <input type="checkbox" checked={shown === true} onChange={(e) => onSet(e.target.checked)} data-widget="bool" />;
  } else if (choices) {
    control = (
      <select value={ref ? "" : text} onChange={(e) => e.target.value && onSet(e.target.value)} data-widget="choice">
        {ref || !text ? <option value="">{ref ? `← ${ref}` : "non défini"}</option> : null}
        {choices.map((c) => <option key={c} value={c}>{c}</option>)}
      </select>
    );
  } else if (p.type === "Int" || p.type === "Float") {
    control = (
      <input type="number" step={p.type === "Int" ? 1 : "any"} defaultValue={ref ? "" : text} placeholder={ref ? `← ${ref}` : "non défini"} data-widget="number"
        onBlur={(e) => e.target.value !== "" && Number(e.target.value) !== shown && onSet(Number(e.target.value))} />
    );
  } else if (p.type === "Json") {
    control = (
      <textarea rows={2} defaultValue={ref ? "" : text} placeholder={ref ? `← ${ref}` : "non défini"} data-widget="json"
        onBlur={(e) => {
          try {
            const v = JSON.parse(e.target.value);
            if (JSON.stringify(v) !== text) onSet(v);
          } catch {
            /* left as typed until it parses */
          }
        }} />
    );
  } else {
    control = (
      <input type="text" defaultValue={ref ? "" : text} placeholder={ref ? `← ${ref}` : "non défini"} data-widget="text"
        onBlur={(e) => e.target.value !== text && e.target.value !== "" && onSet(e.target.value)} />
    );
  }
  return (
    <label className="wf-field" title={p.description ?? ""} data-setting={p.name}>
      <span>
        {p.name}
        {p.required ? " *" : ""} <em>{p.type}</em>
        {ref ? <small className="wf-passed"> ← {ref}</small> : null}
      </span>
      {control}
    </label>
  );
}
