// **The workflow editor's logic** (28 November 2026), without React: what `/ide/graphe` serves, read as nodes with
// typed ports, laid out by layers from left to right. The declared table (`workflow.json`) says which nodes are
// plumbing, the kind a node type reads as, and a port type's colour; nothing of it is written here.
import table from "./workflow.json";

export type PortDecl = { name: string; type: string; required?: boolean };
export type ParamDecl = { name: string; type: string; required?: boolean; default?: unknown; description?: string };
export type NodeSchema = { type: string; description?: string; inputs: PortDecl[]; outputs: PortDecl[]; parameters?: ParamDecl[] };
export type WfNode = { name: string; node_type: string; config?: Record<string, unknown>; line?: number | null; schema?: NodeSchema | null };
export type WfEdge = { from_node: string; from_port: string; to_node: string; to_port: string };
export type WfGraph = {
  kind: string;
  name: string;
  source?: string | null;
  nodes: WfNode[];
  edges: WfEdge[];
  inputSchema?: { properties?: Record<string, { type?: string; default?: unknown; description?: string }>; required?: string[] };
  result?: { node: string; port: string };
  /// A cited graph (`kind: "subgraph"`): the path of graph nodes that leads to it, and what the citing node passes.
  path?: string[];
  passed?: Record<string, unknown>;
};

export type Side = "in" | "out";
export type PortRef = { node: string; side: Side; name: string; type: string };
export const portKey = (p: { node: string; side: Side; name: string }) => `${p.node}:${p.side}:${p.name}`;

type Rule = { node_type: string; config?: Record<string, unknown> };

/// A plumbing node: the Simple view hides it (a collect, a script that only passes its value on).
export function isPlumbing(n: WfNode): boolean {
  return (table.plumbing as Rule[]).some(
    (r) => r.node_type === n.node_type && Object.entries(r.config ?? {}).every(([k, v]) => (n.config ?? {})[k] === v),
  );
}

export const kindOf = (n: WfNode): string => (table.kinds as Record<string, string>)[n.node_type] ?? n.node_type;
export const colorOf = (type: string): string => (table.type_colors as Record<string, string>)[type] ?? table.default_color;
export const isGraphNode = (n: WfNode): boolean => (table.graph_nodes as string[]).includes(n.node_type);

/// The sub-graph a graph node cites (`graph='@graphs/x.mmd'`), or null.
export function subGraphOf(n: WfNode): string | null {
  const g = (n.config ?? {}).graph as unknown;
  // Bound, the route serves the cited graph as `{cite, mermaid}`; unbound, as its text `@graphs/x.mmd`.
  const cite = typeof g === "string" ? g : g && typeof g === "object" ? (g as { cite?: unknown }).cite : null;
  return isGraphNode(n) && typeof cite === "string" ? cite.replace(/^@/, "") : null;
}

/// The node's own settings: the keys of its configuration that are values (a `$param` it takes, or a literal), not
/// its structure (the script, the cited graph, the list of fields). Shown under its ports, as the mock-ups' widgets.
export function settingsOf(n: WfNode): { key: string; text: string }[] {
  const skip = table.structural_keys as string[];
  return Object.entries(n.config ?? {})
    .filter(([k, v]) => !skip.includes(k) && !(n.schema?.inputs ?? []).some((p) => p.name === k) && (typeof v !== "object" || v === null))
    .map(([k, v]) => ({ key: k, text: valueText(v) }));
}

/// Two ports can be joined: an output to an input of another node, of the same type, or one of them takes any.
export function compatible(a: PortRef, b: PortRef): boolean {
  if (a.side === b.side || a.node === b.node) return false;
  const all = table.compatible_with_all as string[];
  return a.type === b.type || all.includes(a.type) || all.includes(b.type);
}

/// A node's ports, outputs first then inputs, as its schema declares them. A port an edge uses that the schema does
/// not declare is kept (typed `?`), so a wire never points nowhere; the editor says so.
export function portsOf(n: WfNode, edges: WfEdge[]): PortRef[] {
  const out: PortRef[] = (n.schema?.outputs ?? []).map((p) => ({ node: n.name, side: "out", name: p.name, type: p.type }));
  const inn: PortRef[] = (n.schema?.inputs ?? []).map((p) => ({ node: n.name, side: "in", name: p.name, type: p.type }));
  for (const e of edges) {
    if (e.from_node === n.name && !out.some((p) => p.name === e.from_port)) out.push({ node: n.name, side: "out", name: e.from_port, type: "?" });
    if (e.to_node === n.name && !inn.some((p) => p.name === e.to_port)) inn.push({ node: n.name, side: "in", name: e.to_port, type: "?" });
  }
  return [...out, ...inn];
}

export const wired = (p: PortRef, edges: WfEdge[]) =>
  edges.some((e) => (p.side === "out" ? e.from_node === p.node && e.from_port === p.name : e.to_node === p.node && e.to_port === p.name));

/// The value a free input reads from the node's own configuration, as text: a `$param` reads as the param it takes.
export function valueText(v: unknown): string {
  if (typeof v === "string") return v.startsWith("$") ? `← ${v.slice(1)}` : v;
  return JSON.stringify(v);
}

/// The line of the `.mmd` a node is declared by, rebuilt from what the route serves.
export function declaration(n: WfNode): string {
  const sub = subGraphOf(n);
  const args = Object.entries(n.config ?? {}).map(([k, v]) =>
    k === "graph" && sub ? `graph='@${sub}'` : `${k}=${typeof v === "string" ? (v.startsWith("$") ? v : `'${v}'`) : JSON.stringify(v)}`,
  );
  return `${n.name}["${n.node_type}(${args.join(", ")})"]`;
}

/// The node's settings shown in the Simple view: the first few, short.
export function detailOf(n: WfNode): string {
  const c = n.config ?? {};
  const sub = subGraphOf(n);
  if (sub) return sub.split("/").pop() ?? sub;
  for (const k of table.detail_keys as string[]) {
    const v = c[k];
    if (typeof v === "string" && v) return v.length > 34 ? v.slice(0, 33) + "…" : v;
  }
  return "";
}

export const HEAD = 46, ROW = 18, W = 200, SIMPLE_H = 86, GAP_X = 70, GAP_Y = 26, M = 20;

/// The rows a node takes in the Edit view: one per port, and one more under a free input that has a value.
export function rowsOf(n: WfNode, edges: WfEdge[]): { port: PortRef; row: number; value?: string }[] {
  const rows: { port: PortRef; row: number; value?: string }[] = [];
  let r = 0;
  for (const p of portsOf(n, edges)) {
    const v = p.side === "in" && !wired(p, edges) ? (n.config ?? {})[p.name] : undefined;
    rows.push({ port: p, row: r, value: v === undefined ? undefined : valueText(v) });
    r += v === undefined ? 1 : 2;
  }
  return rows;
}

/// The rows the ports take, then the first row of the settings.
export function settingsRow(n: WfNode, edges: WfEdge[]): number {
  const rows = rowsOf(n, edges);
  const last = rows[rows.length - 1];
  return last ? last.row + (last.value === undefined ? 1 : 2) : 0;
}

export const MAX_SETTINGS = 6;

export function heightOf(n: WfNode, edges: WfEdge[], edit: boolean): number {
  if (!edit) return SIMPLE_H;
  const settings = settingsOf(n).length;
  const shown = Math.min(settings, MAX_SETTINGS) + (settings > MAX_SETTINGS ? 1 : 0);
  return HEAD + (settingsRow(n, edges) + shown) * ROW + (shown ? 8 : 0) + 10;
}

/// The layers: a node's layer is the longest chain of edges that reaches it (bounded by the number of nodes, so a
/// cycle does not spin), its column; nodes stack in their column.
export function layout(nodes: WfNode[], edges: WfEdge[], edit: boolean) {
  const names = new Set(nodes.map((n) => n.name));
  const shown = edges.filter((e) => names.has(e.from_node) && names.has(e.to_node));
  const layer = new Map(nodes.map((n) => [n.name, 0]));
  for (let i = 0; i < nodes.length; i++)
    for (const e of shown) {
      const c = (layer.get(e.from_node) ?? 0) + 1;
      if (c > (layer.get(e.to_node) ?? 0) && c < nodes.length) layer.set(e.to_node, c);
    }
  const columns: WfNode[][] = [];
  for (const n of nodes) (columns[layer.get(n.name) ?? 0] ??= []).push(n);
  const pos = new Map<string, { x: number; y: number; h: number }>();
  let height = 0;
  columns.forEach((c, i) => {
    let y = M;
    for (const n of c ?? []) {
      const h = heightOf(n, edges, edit);
      pos.set(n.name, { x: M + i * (W + GAP_X), y, h });
      y += h + GAP_Y;
    }
    height = Math.max(height, y);
  });
  return { pos, edges: shown, width: M * 2 + Math.max(1, columns.length) * (W + GAP_X) - GAP_X, height: height + M - GAP_Y };
}

/// The y of a port's dot, inside the canvas.
export function portY(n: WfNode, edges: WfEdge[], at: { y: number }, side: Side, name: string): number {
  const r = rowsOf(n, edges).find((q) => q.port.side === side && q.port.name === name);
  return at.y + HEAD + (r ? r.row : 0) * ROW + ROW / 2;
}
