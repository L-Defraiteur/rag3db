import { useEffect, useState, type ReactNode } from "react";
import { Bot, ChevronDown, ChevronRight, CircleAlert, MessageSquare } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { displayName, fetchPoste, setAsidePrices, setCap, type HostState, type Poste } from "@/lib/host";
import { Button } from "@/components/ui/button";
import { PriceCard } from "@/components/PriceCard";

/// **The cap of the month, raised on the user's yes** (« À intervenir », 28 November 2026): a field and its button.
function CapAction() {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  if (!open) return <Button size="sm" variant="outline" data-action="cap" onClick={() => setOpen(true)}>Relever le plafond</Button>;
  const n = Number(value.replace(",", "."));
  return (
    <span className="inline-flex items-center gap-2" data-action="cap">
      <input className="w-24 rounded border bg-background px-2 py-0.5 text-sm" inputMode="decimal" placeholder="USD / mois" value={value} onChange={(e) => setValue(e.target.value)} aria-label="Nouveau plafond du mois" />
      <Button size="sm" disabled={!value.trim() || !Number.isFinite(n) || n < 0} onClick={() => setCap(n).then(() => location.reload(), (e: Error) => setError(e.message))}>Écrire</Button>
      {error ? <span className="text-destructive text-xs" role="alert">{error}</span> : null}
    </span>
  );
}

/// **An unreadable prices.json set aside** (renamed beside itself, never deleted): the shipped prices take over.
function SetAsideAction() {
  const [error, setError] = useState<string | null>(null);
  return (
    <span className="inline-flex items-center gap-2">
      <Button size="sm" variant="outline" data-action="set-aside" onClick={() => setAsidePrices().then(() => location.reload(), (e: Error) => setError(e.message))}>
        Mettre ce fichier de côté (les prix livrés reprennent)
      </Button>
      {error ? <span className="text-destructive text-xs" role="alert">{error}</span> : null}
    </span>
  );
}

// **The welcome is the tree of the agents** (Lucie, 21 November 2026: « la liste des agents racines, quelles tâches a
// chacun, plans en cours, à coup de dépliage de hiérarchie » ; « ce qui attend une intervention » first). Read from the
// host without a new route: the live sessions of `GET /poste` (by their Claude name), the open tasks of each repository
// (`/ide/taches`, a sub-agent carries `parent_session`), the conversations of `/ide/etat`, and, unfolded, the « doing »
// item of a sub-agent's plan (`/ide/teammate`). `GET /ide/agents` will replace the gluing. Markers of the witness:
// `#needs-attention`, `#agents-tree`, `.agent[data-agent]`, `.toggle`, `.sub-agent`.

type Task = { nom: string; parent_session?: string | null };
type SubAgent = { backend: string; repo: string; task: Task };
type Root = {
  key: string;
  name: string;
  kind: "claude" | "conversation" | "ide";
  active: boolean;
  detail?: string;
  link?: string;
  children: SubAgent[];
};
type Session = Poste["sessions"][number];

export function Agents({ state }: { state: HostState }) {
  const [machine, setMachine] = useState<Poste | null>(null);
  const [tasks, setTasks] = useState<SubAgent[]>([]);
  const mainBackends = (state.backends ?? []).filter((b) => !b.tache && b.kind !== "memory");
  const backendKey = mainBackends.map((b) => b.id).join(",");
  useEffect(() => {
    let alive = true;
    const read = async () => {
      const poste = await fetchPoste().catch(() => null);
      const lists = await Promise.all(
        mainBackends.map(async (b) => {
          try {
            const r = await (await fetch(`/ide/taches?${new URLSearchParams({ backend: b.id })}`)).json();
            const open: Task[] = (r.ok ? r.result?.ouvertes : null) ?? [];
            return open.map((task) => ({ backend: b.id, repo: displayName(b), task }));
          } catch {
            return [];
          }
        }),
      );
      if (alive) {
        setMachine(poste);
        setTasks(lists.flat());
      }
    };
    read();
    const timer = setInterval(read, 5000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [backendKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // The roots: the live Claude sessions, the names sub-agents carry (inactive without a live session), the sessions
  // without a name, the conversations of the repositories, and the tasks launched from the IDE.
  const roots: Root[] = [];
  const live = new Map<string, Session>();
  for (const s of machine?.sessions ?? []) if (s.claude?.name) live.set(s.claude.name, s);
  const parentNames = tasks.map((t) => t.task.parent_session).filter((n): n is string => !!n);
  const names = [...new Set<string>([...live.keys(), ...parentNames])];
  names.sort((a, b) => Number(live.has(b)) - Number(live.has(a)) || a.localeCompare(b));
  for (const name of names) {
    const session = live.get(name);
    roots.push({
      key: `claude:${name}`,
      name,
      kind: "claude",
      active: !!session,
      detail: session ? `${session.client}${session.pid ? ` · pid ${session.pid}` : ""}` : "sans session ouverte",
      children: tasks.filter((t) => t.task.parent_session === name),
    });
  }
  for (const s of machine?.sessions ?? []) {
    if (s.claude?.name || s.client === "ide") continue;
    roots.push({
      key: `unnamed:${s.id}`,
      name: "sans nom",
      kind: "claude",
      active: true,
      detail: `${s.client} : serveur MCP d'avant le 21 novembre, à reconnecter (/mcp)`,
      children: [],
    });
  }
  for (const c of state.chat_sessions ?? []) {
    roots.push({
      key: `conversation:${c.id}`,
      name: c.name || c.id,
      kind: "conversation",
      active: c.state === "running" || c.state === "waiting",
      link: `/ide/code/${c.id}/chat`,
      children: [],
    });
  }
  const unrooted = tasks.filter((t) => !t.task.parent_session);
  if (unrooted.length) roots.push({ key: "ide", name: "lancés depuis l'IDE", kind: "ide", active: true, children: unrooted });

  // What waits for an intervention: a task or a conversation that waits for an answer (a question, an accord).
  // Every line carries its action (Lucie, 28 November 2026): the gesture that settles it, never a sentence alone.
  const answer = (href: string) => (
    <a href={href} data-action="answer" className="inline-flex items-center rounded border px-2 py-0.5 text-xs hover:bg-accent">Répondre</a>
  );
  const alert = state.spending?.alert;
  const waiting: { key: string; text: string; action: ReactNode }[] = [
    // The spending of the month past an alert (50 %, 80 %, the cap) or on course to pass it; or prices that do not read.
    ...(alert ? [{ key: "spending", text: alert.text, action: alert.text.includes("ne se lit pas") ? <SetAsideAction /> : <CapAction /> }] : []),
    // A model refused for want of a price: its price card (the price looked up on its page, then the user's yes).
    ...(state.spending?.unpriced ?? []).map((s) => ({
      key: `price:${s}`,
      text: `le modèle ${s} n'a pas de prix : ses appels sont refusés`,
      action: <PriceCard slug={s} onWritten={() => location.reload()} />,
    })),
    ...(state.tasks ?? [])
      .filter((t) => t.awaits_answer)
      .map((t) => ({ key: `t:${t.id}`, text: `le sous-agent ${t.name} attend une réponse`, action: answer(`/ide/code/${t.session}/taches/${encodeURIComponent(t.name)}`) })),
    ...(state.chat_sessions ?? [])
      .filter((c) => c.awaits_answer)
      .map((c) => ({ key: `c:${c.id}`, text: `la conversation ${c.name || c.id} attend une réponse`, action: answer(`/ide/code/${c.id}/chat`) })),
  ];

  return (
    <>
      <section id="needs-attention" className="mt-4">
        <h2 className="text-base font-semibold mb-2 flex items-center gap-2">
          <CircleAlert className="size-4" /> À intervenir
        </h2>
        {waiting.length ? (
          <ul className="text-sm space-y-1">
            {waiting.map((w) => (
              <li key={w.key} data-waiting={w.key} className="flex flex-wrap items-center gap-x-3 gap-y-1">
                <span>{w.text}</span>
                {w.action}
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">Rien n'attend une intervention.</p>
        )}
      </section>
      <section id="agents-tree" className="mt-6">
        <h2 className="text-base font-semibold mb-2 flex items-center gap-2">
          <Bot className="size-4" /> Agents
        </h2>
        {roots.length ? (
          <ul className="text-sm space-y-1">
            {roots.map((r) => (
              <RootNode key={r.key} root={r} />
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">Aucun agent pour l'instant.</p>
        )}
      </section>
    </>
  );
}

function RootNode({ root }: { root: Root }) {
  const [open, setOpen] = useState(false);
  const hasChildren = root.children.length > 0;
  return (
    <li className="agent" data-agent={root.name}>
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" className="toggle inline-flex" aria-label={open ? "replier" : "déplier"} onClick={() => setOpen(!open)} disabled={!hasChildren}>
          {open ? <ChevronDown className="size-4" /> : <ChevronRight className={`size-4 ${hasChildren ? "" : "opacity-30"}`} />}
        </button>
        {root.kind === "conversation" ? <MessageSquare className="size-4" /> : <Bot className="size-4" />}
        {root.link ? (
          <a href={root.link} className="font-medium hover:underline">{root.name}</a>
        ) : (
          <span className="font-medium">{root.name}</span>
        )}
        <Badge variant={root.active ? "default" : "secondary"}>{root.active ? "actif" : "inactif"}</Badge>
        {hasChildren ? <span className="text-xs text-muted-foreground">{root.children.length} sous-agent(s)</span> : null}
        {root.detail ? <span className="text-xs text-muted-foreground">{root.detail}</span> : null}
      </div>
      {open ? (
        <ul className="ml-6 mt-1 space-y-1">
          {root.children.map((c) => (
            <SubAgentNode key={`${c.backend}/${c.task.nom}`} sub={c} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/// The « doing » item of a plan, wherever the plan keeps its items.
function doingOf(value: unknown): string | null {
  if (!value || typeof value !== "object") return null;
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = doingOf(item);
      if (found) return found;
    }
    return null;
  }
  const record = value as Record<string, unknown>;
  if (record.status === "doing") return String(record.text ?? record.title ?? record.id ?? "en cours");
  for (const item of Object.values(record)) {
    const found = doingOf(item);
    if (found) return found;
  }
  return null;
}

function SubAgentNode({ sub }: { sub: SubAgent }) {
  const [doing, setDoing] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  useEffect(() => {
    fetch(`/ide/teammate?${new URLSearchParams({ backend: sub.backend, task: sub.task.nom })}`)
      .then((r) => r.json())
      .then((r) => {
        if (!r.ok) return;
        setDoing(doingOf(r.result?.plans));
        setRunning(!!r.result?.running);
      })
      .catch(() => {});
  }, [sub.backend, sub.task.nom]);
  return (
    <li className="sub-agent flex flex-wrap items-center gap-2">
      <a href={`/ide/code/${sub.backend}/taches/${encodeURIComponent(sub.task.nom)}`} className="hover:underline">{sub.task.nom}</a>
      <span className="text-xs text-muted-foreground">{sub.repo}</span>
      {running ? <Badge>travaille</Badge> : null}
      {doing ? <span className="text-xs">en cours : {doing}</span> : null}
    </li>
  );
}
