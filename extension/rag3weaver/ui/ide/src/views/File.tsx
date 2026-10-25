import { useEffect, useMemo, useState } from "react";
import { Pencil } from "lucide-react";
import { Button } from "@/components/ui/button";
import { call, onFlux, route, shortName, type FluxEvent, type HostState } from "@/lib/host";
import { Editor } from "@/views/Editor";

// **La vue d'un fichier et sa marge** (page 08, vue 3) : le texte numéroté, chaque
// identifiant menant à son symbole, et « Dans ce fichier » — le plan d'outline
// (chaque scope sous celui qui le contient, vers sa ligne et son symbole, avec sa
// provenance), ou, quand le manifeste n'expose pas `outline`, les scopes que
// `read_file` rend, en disant pourquoi (#plan-repli). « Modifier » ouvre l'édition
// (vue 3b, `Editor`) dans la même page ; un fichier qui change sur le disque
// pendant la lecture se relit.

// Les mots qui ne sont pas des symboles du dépôt (quelques langages courants).
const WORDS = new Set(
  (
    "as async await break case catch class const continue crate default do dyn else enum export extends " +
    "false fn for from function if impl import in interface let loop match mod move mut new null pub ref return self Self " +
    "static struct super switch this throw trait true try type typeof unsafe use var void where while yield int bool char " +
    "float double long short unsigned signed auto template typename namespace using public private protected virtual override " +
    "str String Vec Option Some None Ok Err Result Box Arc usize u8 u16 u32 u64 i32 i64 f32 f64"
  ).split(" "),
);

export const symbolLink = (id: string, name: string) => `/ide/code/${id}/symbole/${encodeURIComponent(name)}`;
export const fileLink = (id: string, path: string, line?: number) => `/ide/code/${id}/fichier/${path.split("/").map(encodeURIComponent).join("/")}${line ? `#L${line}` : ""}`;

type Scope = { name: string; scope_type: string; start_line: number; end_line: number; signature?: string };
type Read = { text: string; total_lines: number; stale: boolean | null; has_more: boolean; offset: number; lines_read: number; scopes?: Scope[] };
type Plan = { rows?: Scope[]; total?: number; provenance?: string; erreur?: string };

/// Une ligne de code : chaque identifiant est un lien vers son symbole.
function ClickableLine({ id, text }: { id: string; text: string }) {
  const parts: React.ReactNode[] = [];
  let last = 0;
  for (const m of text.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) {
    const at = m.index ?? 0;
    if (at > last) parts.push(text.slice(last, at));
    const word = m[0];
    parts.push(WORDS.has(word) || word.length < 2 ? word : <a key={at} className="id" href={symbolLink(id, word)}>{word}</a>);
    last = at + word.length;
  }
  parts.push(text.slice(last));
  return <>{parts}</>;
}

function ScopeRow({ id, s, depth }: { id: string; s: Scope; depth: number }) {
  return (
    <div className="scope py-0.5 border-b border-border/60 text-sm" style={{ ["--profondeur" as string]: depth, paddingLeft: `${depth * 12}px` }} title={s.signature ?? ""} data-ligne={s.start_line}>
      <a href={`#L${s.start_line}`} className="hover:underline">{s.name}</a>{" "}
      <span className="meta note text-xs text-muted-foreground">{s.scope_type} {s.start_line}–{s.end_line}</span>{" "}
      <a href={symbolLink(id, s.name)} title="usages, impact, appelés">→</a>
    </div>
  );
}

/// Le plan d'outline : les scopes dans l'ordre du fichier, chacun sous celui qui le
/// contient (l'imbrication suit les lignes, pas le nom du parent).
function OutlinePlan({ id, plan }: { id: string; plan: Plan }) {
  const rows = plan.rows ?? [];
  const stack: number[] = [];
  const items = rows.map((s, k) => {
    while (stack.length && stack[stack.length - 1] < s.start_line) stack.pop();
    const depth = stack.length;
    stack.push(s.end_line);
    return <ScopeRow key={k} id={id} s={s} depth={depth} />;
  });
  return (
    <>
      {items.length ? items : <p className="note text-muted-foreground text-sm">aucun scope</p>}
      {typeof plan.total === "number" && plan.total > rows.length ? <p className="note text-xs text-muted-foreground" id="plan-borne">{rows.length} scopes montrés sur {plan.total}</p> : null}
      {plan.provenance ? <p className="note text-xs text-muted-foreground" id="plan-provenance">{plan.provenance}</p> : null}
    </>
  );
}

/// Le repli : les scopes de read_file, sans imbrication ni provenance, et la raison.
function ReadPlan({ id, scopes, why }: { id: string; scopes: Scope[]; why: string }) {
  return (
    <>
      {scopes.length ? scopes.map((s, k) => <ScopeRow key={k} id={id} s={s} depth={0} />) : <p className="note text-muted-foreground text-sm">aucun scope</p>}
      <p className="note text-xs text-muted-foreground" id="plan-repli">scopes lus avec le fichier : le plan d'outline n'est pas disponible ({why})</p>
    </>
  );
}

export function File({ id, path, state }: { id: string; path: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [read, setRead] = useState<Read | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [generation, setGeneration] = useState(0);
  const offset = Number(new URLSearchParams(location.search).get("depuis") ?? 1) || 1;

  // En lecture, le fichier qui change sur le disque se relit (un peu après).
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const off = onFlux((d: FluxEvent) => {
      const ev = d.evenement ?? {};
      if (editing || !backend || d.backend !== backend.manifest || ev.kind !== "FileChanged" || ev.path !== path) return;
      clearTimeout(timer);
      timer = setTimeout(() => setGeneration((g) => g + 1), 300);
    });
    return () => { off(); clearTimeout(timer); };
  }, [backend?.manifest, path, editing]);

  useEffect(() => {
    if (!backend || editing) return;
    setRead(null);
    setPlan(null);
    Promise.all([
      call(backend.manifest, "read_file", { path, offset, limit: 2000, format: "json" }) as Promise<Read>,
      (call(backend.manifest, "outline", { path, format: "json" }) as Promise<unknown>)
        .then((t) => (typeof t === "string" ? JSON.parse(t) : t) as Plan)
        .catch((e: Error) => ({ erreur: e.message }) as Plan),
    ])
      .then(([r, p]) => {
        setRead(r);
        setPlan(p);
      })
      .catch((e: Error) => setError(e.message));
  }, [backend?.manifest, path, offset, editing, generation]);

  // La ligne visée par l'ancre (#L12), centrée, à l'arrivée et au changement d'ancre.
  useEffect(() => {
    const aim = () => {
      document.querySelectorAll("tr.vise").forEach((t) => t.classList.remove("vise"));
      const target = location.hash && document.getElementById(location.hash.slice(1));
      if (target) {
        target.classList.add("vise");
        target.scrollIntoView({ block: "center" });
      }
    };
    aim();
    addEventListener("hashchange", aim);
    return () => removeEventListener("hashchange", aim);
  }, [read]);

  const lines = useMemo(() => {
    const out: { n: number; text: string }[] = [];
    for (const raw of (read?.text ?? "").split("\n")) {
      const m = raw.match(/^(\d+)\| ?(.*)$/);
      if (m) out.push({ n: Number(m[1]), text: m[2] });
    }
    return out;
  }, [read]);

  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  if (editing) return <Editor manifest={backend.manifest} path={path} onClose={() => setEditing(false)} />;
  const parts = path.split("/").filter(Boolean);
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href={`/ide/code/${id}/arbre/`} className="hover:underline">{shortName(backend.manifest)}</a>
        {parts.map((p, i) => {
          const sub = parts.slice(0, i + 1).join("/");
          const last = i === parts.length - 1;
          return (
            <span key={sub}>
              {" / "}
              <a href={last ? fileLink(id, sub) : `/ide/code/${id}/arbre/${sub.split("/").map(encodeURIComponent).join("/")}/`} className="hover:underline">{p}</a>
            </span>
          );
        })}
      </p>
      <h1 className="text-xl font-semibold mb-2 break-all">{path}</h1>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {read ? (
        <>
          <p className="barre flex items-center gap-3 mb-2">
            <Button variant="outline" size="sm" onClick={() => setEditing(true)} data-modifier>
              <Pencil className="size-3.5" />Modifier
            </Button>
            <span className="note text-xs text-muted-foreground">
              {read.total_lines} lignes
              {read.stale === true ? <span className="alerte text-destructive"> · modifié depuis l'index</span> : read.stale === null ? " · non indexé" : ""}
              {read.has_more ? <> · <a href={`${location.pathname}?depuis=${read.offset + read.lines_read}`} className="underline">la suite</a></> : null}
            </span>
          </p>
          <div className="grille grid gap-6 md:grid-cols-[1fr_280px] items-start">
            <div className="code min-w-0 overflow-x-auto rounded-lg border bg-card">
              <table className="w-full text-[13px] font-mono leading-5">
                <tbody>
                  {lines.map((l) => (
                    <tr key={l.n} id={`L${l.n}`} className="[&.vise]:bg-accent">
                      <td className="n select-none text-right pr-3 pl-2 text-muted-foreground align-top w-10">
                        <a href={`#L${l.n}`}>{l.n}</a>
                      </td>
                      <td className="whitespace-pre align-top pr-3">
                        <ClickableLine id={id} text={l.text} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <aside id="plan" className="md:sticky md:top-14 max-h-[80dvh] overflow-auto">
              <h2 className="text-sm font-semibold mb-2">Dans ce fichier</h2>
              {plan ? plan.erreur ? <ReadPlan id={id} scopes={read.scopes ?? []} why={plan.erreur} /> : <OutlinePlan id={id} plan={plan} /> : null}
            </aside>
          </div>
        </>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}

export function fileRoute(): { id: string; path: string } | null {
  const r = route();
  return r.view === "fichier" && r.id ? { id: r.id, path: r.rest ?? "" } : null;
}
