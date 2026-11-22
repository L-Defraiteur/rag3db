import { useEffect, useState } from "react";

// **The left bar of the agents** (Lucie, 22 November 2026, 23 h 4x: « juste la liste des agents auxquels j'ai accès, peu
// importe l'hôte »). Read from `GET /ide/agents` (arbre's contract: `chats`, `roots` and their `tasks`, `unrooted`). One
// line per agent, on two levels: its name, its state, its unread; then since when and what it did (its step if it has a
// plan, else its last action; the command on hover). Those who wait for you first, then the active ones, then the
// others, latest first. Inside an agent (the page is its `link`): « ← Retour », its sub-agents, then the agents it
// talked with in the hour. Markers of the witness: `#agents-bar`, `.agent-row[data-agent]`, `.back`.

type Activity = { at?: number | null; phrase?: string | null; command?: string | null };
type Contact = { id: string; last_message_at?: number | null; last_message?: { from?: string; text?: string } | null };
export type AgentNode = {
  id: string;
  name?: string | null;
  state?: "active" | "paused" | "stopped" | "waiting" | string;
  machine?: string | null;
  parent?: string | null;
  children?: string[];
  last_activity?: Activity | null;
  unread?: number;
  pending?: number;
  contacts?: Contact[];
  plan?: { current: number; total: number; title: string } | null;
  link?: string | null;
  tasks?: AgentNode[];
  lifecycle?: string;
};
type AgentsReply = { chats?: AgentNode[]; roots?: AgentNode[]; unrooted?: AgentNode[] };

const DOT: Record<string, { mark: string; words: string }> = {
  waiting_you: { mark: "◐", words: "t'attend" },
  active: { mark: "●", words: "actif" },
  paused: { mark: "‖", words: "en pause" },
  stopped: { mark: "○", words: "à l'arrêt" },
  waiting: { mark: "○", words: "en attente" },
};

function since(at?: number | null): string {
  if (!at) return "";
  const s = Math.max(0, Math.round((Date.now() - at) / 1000));
  if (s < 60) return `il y a ${s} s`;
  if (s < 3600) return `il y a ${Math.round(s / 60)} min`;
  if (s < 86400) return `il y a ${Math.round(s / 3600)} h`;
  return `il y a ${Math.round(s / 86400)} j`;
}
function short(text: string, n = 34): string {
  return text.length > n ? `${text.slice(0, n - 1)}…` : text;
}
function stateOf(a: AgentNode): string {
  return (a.pending ?? 0) > 0 ? "waiting_you" : String(a.state ?? "stopped");
}
function secondLine(a: AgentNode): string {
  if (a.plan) return `étape ${a.plan.current}/${a.plan.total} · ${a.plan.title}`;
  return a.last_activity?.phrase ?? "";
}
function rank(a: AgentNode): number {
  return { waiting_you: 0, active: 1 }[stateOf(a)] ?? 2;
}

function flatten(r: AgentsReply): AgentNode[] {
  const all: AgentNode[] = [...(r.chats ?? [])];
  for (const root of r.roots ?? []) {
    all.push(root);
    for (const t of root.tasks ?? []) if (t.lifecycle === undefined || t.lifecycle === "open") all.push(t);
  }
  for (const t of r.unrooted ?? []) if (t.lifecycle === undefined || t.lifecycle === "open") all.push(t);
  const seen = new Set<string>();
  return all
    .filter((a) => (seen.has(a.id) ? false : (seen.add(a.id), true)))
    .sort((a, b) => rank(a) - rank(b) || (b.last_activity?.at ?? 0) - (a.last_activity?.at ?? 0));
}

function Row({ a, line, title }: { a: AgentNode; line?: string; title?: string }) {
  const st = DOT[stateOf(a)] ?? DOT.stopped;
  const machines = a.machine ? <span className="text-[10px] text-muted-foreground ml-1">{a.machine}</span> : null;
  const head = (
    <>
      <span className="flex items-center gap-1.5">
        <span aria-label={st.words} title={st.words}>{st.mark}</span>
        <span className="truncate font-medium">{a.name ?? ""}</span>
        {machines}
        {(a.unread ?? 0) > 0 ? <span className="ml-auto rounded-full bg-primary px-1.5 text-[10px] text-primary-foreground">{a.unread}</span> : null}
      </span>
      <span className="block truncate text-[11px] text-muted-foreground" title={title}>
        {[since(a.last_activity?.at), short(line ?? secondLine(a))].filter(Boolean).join(" · ")}
      </span>
    </>
  );
  return (
    <li className="agent-row" data-agent={a.id}>
      {a.link ? (
        <a href={a.link} className="block rounded-md px-2 py-1.5 hover:bg-muted">{head}</a>
      ) : (
        <span className="block px-2 py-1.5 opacity-80" title="une session du terminal">{head}</span>
      )}
    </li>
  );
}

export function AgentsBar() {
  const [reply, setReply] = useState<AgentsReply | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () =>
      fetch("/ide/agents")
        .then((r) => r.json())
        .then((v) => alive && setReply((v?.result ?? v) as AgentsReply))
        .catch(() => undefined);
    read();
    const t = setInterval(read, 5000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, []);
  const agents = reply ? flatten(reply) : [];
  const here = agents.find((a) => a.link && (location.pathname === a.link || location.pathname.startsWith(`${a.link}/`)));
  const byId = new Map(agents.map((a) => [a.id, a]));
  return (
    <aside id="agents-bar" aria-label="Agents" className="block w-56 shrink-0 border-r overflow-y-auto px-2 py-3 text-sm">
      {here ? (
        <>
          <a href="/ide/" className="back flex items-center gap-1 px-2 py-1 text-muted-foreground hover:text-foreground">
            ← Retour
            {agents.some((a) => a !== here && stateOf(a) === "waiting_you") ? <span title="un agent t'attend ailleurs">◐</span> : null}
          </a>
          <ul className="mt-2">
            <Row a={here} />
          </ul>
          {(here.children ?? []).length ? (
            <>
              <p className="mt-3 px-2 text-[11px] uppercase tracking-wide text-muted-foreground">Ses agents</p>
              <ul>{(here.children ?? []).map((c) => byId.get(c)).filter(Boolean).map((c) => <Row key={c!.id} a={c!} />)}</ul>
            </>
          ) : null}
          {(here.contacts ?? []).length ? (
            <>
              <p className="mt-3 px-2 text-[11px] uppercase tracking-wide text-muted-foreground">En contact</p>
              <ul>
                {(here.contacts ?? []).map((c) => {
                  const other = byId.get(c.id);
                  if (!other) return null;
                  const text = c.last_message?.text ?? "";
                  const arrow = c.last_message?.from === here.id ? "→ " : "← ";
                  return <Row key={c.id} a={{ ...other, last_activity: { at: c.last_message_at ?? null } }} line={text ? arrow + text : ""} title={text} />;
                })}
              </ul>
            </>
          ) : null}
        </>
      ) : (
        <ul>{agents.map((a) => <Row key={a.id} a={a} title={a.last_activity?.command ?? undefined} />)}</ul>
      )}
    </aside>
  );
}
