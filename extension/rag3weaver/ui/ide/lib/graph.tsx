// **Le dessin d'un graphe** (la forme de /ide/graphe), par couches de gauche à
// droite : la couche d'un nœud est la plus longue chaîne d'arêtes qui y mène
// (bornée par le nombre de nœuds : un cycle ne fait pas tourner la page).
// `classOf(nom)` et `edgeClass(e)` ajoutent leurs classes (le diff d'une
// proposition : ajoute, change, retire ; le DAG vivant : passe, echec). Les
// repères sont ceux de l'ancienne page : `svg g.etape[data-nom]`, `text.duree`.
import type { ReactNode } from "react";

export type GraphNode = { name: string; node_type: string; line?: number; config?: unknown };
export type GraphEdge = { from_node: string; from_port: string; to_node: string; to_port: string };
export type Graph = { nodes: GraphNode[]; edges: GraphEdge[]; source?: string };

export const short = (s: string, n: number) => (s.length > n ? s.slice(0, n - 1) + "…" : s);

export function layout(g: Graph) {
  const layer = new Map(g.nodes.map((n) => [n.name, 0]));
  for (let i = 0; i < g.nodes.length; i++)
    for (const e of g.edges) {
      const c = (layer.get(e.from_node) ?? 0) + 1;
      if (layer.has(e.to_node) && c > layer.get(e.to_node)! && c < g.nodes.length) layer.set(e.to_node, c);
    }
  const columns: GraphNode[][] = [];
  for (const n of g.nodes) (columns[layer.get(n.name) ?? 0] ??= []).push(n);
  const L = 250, H = 56, W = 200, M = 24;
  const height = Math.max(1, ...Array.from(columns, (c) => c?.length ?? 0)) * H + 2 * M;
  const width = Math.max(1, columns.length) * L + 2 * M;
  const pos = new Map<string, { x: number; y: number }>();
  columns.forEach((c, x) => (c ?? []).forEach((n, i) => pos.set(n.name, { x: M + x * L, y: M + i * H })));
  return { pos, width, height, L, H, W };
}

export function GraphSvg({ g, classOf = () => "", edgeClass = () => "", durations }: { g: Graph; classOf?: (name: string) => string; edgeClass?: (e: GraphEdge) => string; durations?: Map<string, string> }) {
  const { pos, width, height, H, W } = layout(g);
  const edges: ReactNode[] = [];
  g.edges.forEach((e, k) => {
    const a = pos.get(e.from_node);
    const z = pos.get(e.to_node);
    if (!a || !z) return;
    const x1 = a.x + W, y1 = a.y + (H - 12) / 2, x2 = z.x, y2 = z.y + (H - 12) / 2;
    edges.push(
      <g key={k}>
        <path d={`M${x1},${y1} C${(x1 + x2) / 2},${y1} ${(x1 + x2) / 2},${y2} ${x2},${y2}`} className={edgeClass(e)} fill="none" stroke="currentColor" strokeOpacity={0.5} />
        <text x={(x1 + x2) / 2} y={(y1 + y2) / 2 - 3} textAnchor="middle" className="port" fontSize={10} fill="currentColor" opacity={0.7}>
          {e.from_port === e.to_port ? e.from_port : `${e.from_port}→${e.to_port}`}
        </text>
      </g>,
    );
  });
  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="graphe-svg text-foreground">
      <g>{edges}</g>
      <g>
        {g.nodes.map((n) => {
          const p = pos.get(n.name)!;
          return (
            <g key={n.name} className={`etape ${classOf(n.name)}`.trim()} data-nom={n.name} transform={`translate(${p.x},${p.y})`}>
              <title>{`${n.node_type}${n.line ? " — ligne " + n.line : ""}\n${JSON.stringify(n.config)}`}</title>
              <rect width={W} height={H - 12} rx={5} className="fill-card stroke-border" strokeWidth={1} />
              <text x={8} y={14} className="titre" fontSize={12} fontWeight={600} fill="currentColor">{short(n.name, 28)}</text>
              <text x={8} y={27} className="lieu" fontSize={10} fill="currentColor" opacity={0.7}>{short(n.node_type, 30)}</text>
              <text x={W - 8} y={27} textAnchor="end" className="duree" fontSize={10} fill="currentColor">{durations?.get(n.name) ?? ""}</text>
            </g>
          );
        })}
      </g>
    </svg>
  );
}

/// Un graphe touché, avant et après, avec son diff : la réunion des deux, les
/// nœuds ajoutés, changés et retirés surlignés (les propositions, la relecture).
export type GraphDiff = { cible: Record<string, string>; diff: { ajoutes: string[]; changes: string[]; retires: string[]; aretes_ajoutees: string[]; aretes_retirees: string[] }; avant?: Graph; apres?: Graph };
export function GraphDiffSection({ g }: { g: GraphDiff }) {
  const d = g.diff;
  const key = (e: GraphEdge) => `${e.from_node}.${e.from_port}→${e.to_node}.${e.to_port}`;
  const union: Graph = {
    nodes: [...(g.apres?.nodes ?? []), ...(g.avant?.nodes ?? []).filter((n) => d.retires.includes(n.name))],
    edges: [...(g.apres?.edges ?? []), ...(g.avant?.edges ?? []).filter((e) => d.aretes_retirees.includes(key(e)))],
  };
  const [genre, nom] = Object.entries(g.cible)[0] ?? ["", ""];
  return (
    <section className="graphe-propose rounded-lg border px-3 py-2" data-cible={`${genre}:${nom}`}>
      <h2 className="text-base font-semibold">graphe — {genre} {nom}</h2>
      <p className="note text-xs text-muted-foreground">
        {d.ajoutes.length} ajouté(s), {d.changes.length} changé(s), {d.retires.length} retiré(s) ; {d.aretes_ajoutees.length} arête(s) ajoutée(s), {d.aretes_retirees.length} retirée(s).
      </p>
      <div className="graphe overflow-x-auto">
        <GraphSvg g={union} classOf={(n) => (d.ajoutes.includes(n) ? "ajoute" : d.changes.includes(n) ? "change" : d.retires.includes(n) ? "retire" : "")} edgeClass={(e) => (d.aretes_ajoutees.includes(key(e)) ? "ajoutee" : d.aretes_retirees.includes(key(e)) ? "retiree" : "")} />
      </div>
    </section>
  );
}
