import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Square, SendHorizontal, MessageSquare, FileDiff, Scale, GitBranch } from "lucide-react";
import { Button } from "@/components/ui/button";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { displayName, fetchChat, onFlux, sendChat, stopRun, type Backend, type ChatContext, type ChatState, type ChatTurn, type FluxEvent, type HostState, type MemoryWrite, type TurnResult } from "@/lib/host";

// **Le chat de la tâche et de l'atelier** (page 08, vue 2) : la forme de ui/chat —
// la colonne des conversations, l'en-tête nom / modèle / état, le fil, la saisie
// en bas (Entrée envoie, Maj + Entrée saute une ligne), la reprise après
// rechargement — branchée sur le flux du chat de l'hôte ; plus les outils
// dépliés sous la réponse pendant le flux, le lien vers la relecture quand un
// tour a écrit, et le lien « bilans ». Un seul composant pour les deux chats.

// **Le Markdown des réponses** (c7, 25 octobre 2026) : `marked` le rend, `dompurify`
// l'assainit — le texte vient d'un modèle, un `<script>` ou un gestionnaire
// d'événement y sont neutralisés. Le message de la personne reste du texte.
marked.setOptions({ gfm: true, breaks: true });
function renderMarkdown(text: string): string {
  // Rogné : marked finit par un retour à la ligne, qui ferait un nœud texte de plus
  // dans `.body` (un témoin compare son texte à l'égal).
  return DOMPurify.sanitize(marked.parse(text, { async: false }) as string, { USE_PROFILES: { html: true } }).trim();
}

// Un appel d'outil dans une bulle : son départ (`at`) et son retour (`end`) en ms
// epoch, quand on les sait ; `result` est l'extrait du flux (`truncated`) puis le
// résultat entier relu dans la session à la fin du tour.
type ToolBlock = { id: string; name: string; args: string; result?: string; ok?: boolean; truncated?: boolean; at?: number; end?: number };
// **Une bulle d'agent = ses parties dans l'ordre d'arrivée** (ticket « les outils
// changent de place quand la réponse arrive ») : mots, appel, mots, appel… la
// réponse à la fin ; le flux et la relecture rendent la même suite.
type Part = { kind: "text"; text: string } | ({ kind: "tool" } & ToolBlock);
type Message = { kind: "message"; role: "user" | "assistant" | "error"; parts: Part[]; run?: string; live?: boolean; propositions: TurnResult["propositions"]; task?: { tache: NonNullable<TurnResult["tache"]>; fichiers: string[] } };
type Item = Message | { kind: "info"; text: string };
type TokenLog = { received: Record<string, number[]>; applied: Record<string, number[]> };

const textOf = (m: Message) => m.parts.filter((p) => p.kind === "text").map((p) => (p as { text: string }).text).join("");
const message = (role: Message["role"], text: string, extra: Partial<Message> = {}): Message => ({ kind: "message", role, parts: text ? [{ kind: "text", text }] : [], propositions: [], ...extra });

/// Un jeton prolonge la dernière partie texte, ou en ouvre une après un outil.
function appendText(parts: Part[], text: string): Part[] {
  if (!text) return parts;
  const last = parts[parts.length - 1];
  if (last?.kind === "text") return [...parts.slice(0, -1), { kind: "text", text: last.text + text }];
  return [...parts, { kind: "text", text }];
}

/// **La relecture rend la suite du flux** : les tours d'assistant consécutifs
/// d'une même réponse (ceux qu'aucun message de la personne ne sépare) se fondent
/// dans une bulle, chacun son texte puis ses appels ; un tour `tool` rattache son
/// résultat et son heure à l'appel qu'il clôt.
function fromTurns(turns: ChatTurn[]): Item[] {
  const items: Item[] = [];
  const byCall = new Map<string, { kind: "tool" } & ToolBlock>();
  let current: Message | null = null;
  for (const t of turns) {
    if (t.role === "tool") {
      const b = byCall.get(t.tool_call_id ?? "");
      if (b) {
        b.result = t.content ?? "";
        b.end = t.at_ms;
      }
      continue;
    }
    if (t.role === "user") {
      current = null;
      items.push(message("user", t.content ?? ""));
      continue;
    }
    if (t.role !== "assistant") continue;
    if (!current) {
      current = message("assistant", "");
      items.push(current);
    }
    current.parts = appendText(current.parts, t.content ?? "");
    for (const c of t.tool_calls ?? []) {
      const b = { kind: "tool" as const, id: c.id, name: c.name, args: c.arguments, at: c.at_ms };
      byCall.set(c.id, b);
      current.parts.push(b);
    }
  }
  return items.filter((i) => i.kind !== "message" || i.parts.length);
}

/// L'heure d'un appel et sa durée, quand on les sait.
function when(t: ToolBlock): string {
  if (t.at === undefined) return "";
  const at = new Date(t.at).toLocaleTimeString();
  return t.end !== undefined ? `${at} · ${t.end - t.at} ms` : at;
}

/// Les jetons lisibles : 12 345 → « 12,3 k ».
const tokens = (n: number | null | undefined) => (n == null ? "?" : n >= 1000 ? `${(n / 1000).toFixed(1).replace(/\.0$/, "")} k` : String(n));

/// Une ligne de la barre des runs à rôle : l'archiviste, la compaction…
type RoleRun = { run: string; role: string; what: string; at?: number; state: "en cours" | "fini" | "en erreur" };
/// Ce qu'une conversation de la barre montre sous son nom.
type Recap = { context?: ChatContext; model?: string | null; writes?: MemoryWrite[]; writesWhy?: string | null };

/// **La jauge du contexte** : le dernier appel au modèle sur le maximum du modèle
/// déclaré ; « inconnu » quand l'un manque, jamais une valeur codée en dur.
function Gauge({ context, model, counters }: { context?: ChatContext; model?: string | null; counters?: string }) {
  const used = context?.used ?? null;
  const max = context?.max ?? null;
  const ratio = used != null && max ? Math.min(1, used / max) : null;
  return (
    <div className="usage mt-1 mb-2 pl-6 pr-1 text-[11px] text-muted-foreground" data-used={used ?? ""} data-max={max ?? ""}>
      <div className="h-1.5 rounded bg-sidebar-accent overflow-hidden" role="meter" aria-label="contexte utilisé" aria-valuemin={0} aria-valuemax={max ?? undefined} aria-valuenow={used ?? undefined}>
        <div className="h-full bg-primary" style={{ width: `${Math.round((ratio ?? 0) * 100)}%` }} />
      </div>
      <div className="flex justify-between gap-2 mt-0.5">
        <span className="contexte">{used == null ? "contexte : pas encore de tour" : `contexte ${tokens(used)} / ${max ? tokens(max) : "inconnu"}`}</span>
        {model ? <span className="truncate">{model}</span> : null}
      </div>
      {counters ? <div id="chat-conso" className="break-words">{counters}</div> : null}
    </div>
  );
}

/// **Les fiches écrites pendant la conversation**, repliées sauf pour la
/// conversation de la page : le nom (que `get_fiche` lit), ce qui a été écrit,
/// l'heure.
function Writes({ writes, why, open }: { writes?: MemoryWrite[]; why?: string | null; open: boolean }) {
  const list = writes ?? [];
  return (
    <details className="fiches-ecrites pl-6 mb-2 text-[11px] text-muted-foreground" open={open || undefined}>
      <summary className="cursor-pointer">fiches écrites ({list.length})</summary>
      {list.length ? (
        <ul className="mt-1 flex flex-col gap-0.5">
          {list.map((w, k) => (
            <li key={k} className="fiche-ecrite flex justify-between gap-2" data-fiche={w.fiche} data-run={w.run}>
              <span className="truncate font-mono" title={`get_fiche « ${w.fiche} »`}>
                {w.fiche}
              </span>
              <span className="shrink-0">
                {w.kind === "apport" ? (w.section ? w.section : "apport") : "fiche"}
                {w.at_ms ? ` · ${new Date(w.at_ms).toLocaleTimeString()}` : ""}
              </span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-1 pourquoi">{why ?? "aucune pour l'instant"}</p>
      )}
      {list.length && why ? <p className="mt-1 pourquoi">{why}</p> : null}
      <p className="mt-1 italic">les écritures de l'agent lui-même arrivent avec M7 de la mémoire longue</p>
    </details>
  );
}

export function Chat({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [chat, setChat] = useState<ChatState | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [status, setStatus] = useState("Prêt");
  const [busy, setBusy] = useState(false);
  const [run, setRun] = useState<string | null>(null);
  const [consumed, setConsumed] = useState("");
  const [recaps, setRecaps] = useState<Record<string, Recap>>({});
  const [roleRuns, setRoleRuns] = useState<RoleRun[]>([]);
  // La saisie n'est pas contrôlée par React : un témoin (ou une saisie
  // automatique du navigateur) pose la valeur par le DOM, et `send` la lit là.
  const input = useRef<HTMLTextAreaElement>(null);
  const messages = useRef<HTMLElement>(null);
  const runRef = useRef<string | null>(null);
  runRef.current = run;
  // **Le défilement** (tickets « le chat saute au scroll », « déplier un outil
  // déplace le scroll ») : c'est l'état du dernier défilement de la personne qui
  // décide. Collée en bas (à moins de 40 px), la liste suit le flux, sans
  // animation ; remontée, elle ne bouge plus. Seul un événement du flux ou un
  // envoi demande de suivre (`follow`) ; déplier un outil ne recolle jamais.
  const stuck = useRef(true);
  const follow = useRef(false);

  // **Toutes les conversations de l'hôte** (Lucie, 7 novembre 2026) : chaque
  // backend ouvert, et celui de la page même s'il ne l'est pas encore.
  const backendsHere: Backend[] = state.backends.filter((b) => b.open || b.id === id);

  // La jauge de chaque conversation de la barre : son propre état du chat.
  const reloadRecaps = () =>
    Promise.all(
      backendsHere.map((b) =>
        fetchChat(b.id)
          .then((c) => [b.id, { context: c.context, model: c.model, writes: c.memory_writes, writesWhy: c.memory_writes_why }] as const)
          .catch(() => [b.id, {}] as const),
      ),
    ).then((all) => setRecaps(Object.fromEntries(all)));

  // L'état du chat, à l'ouverture et à chaque réouverture du flux : un tour parti
  // avant (page rechargée, flux coupé puis repris) a publié son run.
  const reload = async () => {
    const s = await fetchChat(id);
    setChat(s);
    follow.current = true;
    setItems((cur) => (cur.some((i) => i.kind === "message" && i.live) ? cur : fromTurns(s.turns)));
    if (s.run) {
      setRun(s.run);
      setBusy(true);
      setStatus("l'agent travaille…");
      setItems((cur) => (cur.some((i) => i.kind === "message" && i.live) ? cur.map((i) => (i.kind === "message" && i.live ? { ...i, run: s.run! } : i)) : [...cur, message("assistant", "", { run: s.run!, live: true })]));
    }
    reloadRecaps();
  };
  useEffect(() => {
    reload().catch((e: Error) => setStatus(e.message));
    const w = window as unknown as { __ide?: { flux?: EventSource; relireLeChat?: () => Promise<void> } };
    if (w.__ide) w.__ide.relireLeChat = reload;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  // Le flux : les jetons, les appels d'outil, les propositions du run en cours ;
  // et, de tous les backends, les runs dont l'outil ou la réaction a un rôle.
  useEffect(() => {
    const live = (f: (m: Message) => Message) => {
      follow.current = true;
      setItems((cur) => cur.map((i) => (i.kind === "message" && i.live ? f(i) : i)));
    };
    // **Le relevé des jetons** (ticket du premier jeton perdu en direct) : par
    // run, les rangs reçus du flux avant tout filtre, puis ceux que la page a
    // posés dans la bulle. Un rang 0 absent des reçus accuse le flux ; reçu mais
    // absent des posés, la page. Lu par les témoins, jamais par la page.
    const tokenLog = ((window as unknown as { __chatTokens?: TokenLog }).__chatTokens ??= { received: {}, applied: {} });
    return onFlux((d: FluxEvent) => {
      const ev = d.evenement ?? {};
      if (ev.kind === "Token" && typeof ev.seq === "number") (tokenLog.received[String(ev.run)] ??= []).push(ev.seq);
      // Le flux rouvert (page à peine chargée, flux coupé puis repris) : l'état
      // du chat se relit, avant tout filtre — ce signal n'a pas de backend.
      if (d.sujet === "flux" && ev.kind === "ouvert") {
        reload().catch(() => {});
        return;
      }
      // **Les runs à rôle** (forme de 96 et d'everything declarative) : un
      // `RunStarted` de graphe dont la cible déclare un rôle ; sa fin par `run`.
      const target = ev.target as { tool?: string; reaction?: string; role?: string } | undefined;
      if (ev.kind === "RunStarted" && ev.run_kind === "graph" && target?.role) {
        const r: RoleRun = { run: String(ev.run), role: target.role, what: target.tool ?? target.reaction ?? "", at: d.at_ms, state: "en cours" };
        setRoleRuns((cur) => [r, ...cur.filter((x) => x.run !== r.run)].slice(0, 12));
      } else if (ev.kind === "RunFinished") {
        setRoleRuns((cur) => cur.map((x) => (x.run === ev.run ? { ...x, state: ev.ok === false ? "en erreur" : "fini" } : x)));
      }
      if (!backend || d.backend !== backend.manifest) return;
      if (ev.kind === "RunStarted" && ev.run_kind === "agent" && ev.name === `chat:${id}`) {
        setRun(String(ev.run));
        setBusy(true);
        live((m) => ({ ...m, run: String(ev.run) }));
        return;
      }
      if (ev.run !== undefined && runRef.current && ev.run !== runRef.current) return;
      if (ev.kind === "Token") {
        if (typeof ev.seq === "number") (tokenLog.applied[String(ev.run)] ??= []).push(ev.seq);
        live((m) => ({ ...m, parts: appendText(m.parts, String(ev.text ?? "")) }));
      }
      else if (ev.kind === "ToolCallStarted") {
        setStatus(String(ev.tool));
        live((m) => ({ ...m, parts: [...m.parts, { kind: "tool", id: String(ev.call_id), name: String(ev.tool), args: String(ev.arguments ?? ""), at: d.at_ms }] }));
      } else if (ev.kind === "ToolCallFinished") {
        const ok = ev.ok !== false;
        const excerpt = typeof ev.result_excerpt === "string" ? ev.result_excerpt : undefined;
        live((m) => ({
          ...m,
          parts: m.parts.map((p) =>
            p.kind === "tool" && p.id === String(ev.call_id)
              ? { ...p, ok, result: excerpt ?? (ok ? "" : `refusé (${ev.error_kind ?? "erreur"})`), truncated: ev.truncated === true, end: p.at !== undefined && typeof ev.ms === "number" ? p.at + ev.ms : d.at_ms }
              : p,
          ),
        }));
      } else if (ev.kind === "PropositionChanged" && ev.change === "proposed") {
        live((m) => (m.propositions.some((p) => p.id === ev.id) ? m : { ...m, propositions: [...m.propositions, { id: Number(ev.id), paths: ev.paths as string[] | undefined, accepted: ev.accepted as boolean | undefined }] }));
      } else if (ev.kind === "RunFinished" && ev.run === runRef.current) {
        setRun(null);
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, backend?.manifest]);

  // Recoller en bas, seulement si on le demande et si la personne y était.
  useLayoutEffect(() => {
    const m = messages.current;
    if (m && follow.current && stuck.current) m.scrollTop = m.scrollHeight;
    follow.current = false;
  }, [items]);

  const send = async () => {
    const text = input.current?.value.trim() ?? "";
    if (!text || busy || !chat?.on) return;
    if (input.current) input.current.value = "";
    setBusy(true);
    setStatus("l'agent travaille…");
    setConsumed("");
    stuck.current = true;
    follow.current = true;
    setItems((cur) => [...cur, message("user", text), message("assistant", "", { live: true })]);
    try {
      const r = await sendChat(id, text);
      // Le tour fini : les résultats entiers et les heures se relisent dans la
      // session, et se rattachent par l'appel — la bulle garde sa suite.
      const s = await fetchChat(id).catch(() => null);
      // **La session fait foi à la fin du tour** : la bulle prend la suite relue
      // (la dernière bulle d'agent de la session), résultats entiers et heures
      // compris — le flux n'a servi qu'à montrer le tour pendant qu'il se faisait.
      // Un jeton manqué en route (vu le 7 novembre sous charge : le premier jeton
      // du premier tour) ne laisse donc pas une réponse tronquée à l'écran.
      const relues = s ? fromTurns(s.turns) : [];
      const derniere = relues[relues.length - 1];
      const relue = derniere && derniere.kind === "message" && derniere.role === "assistant" ? derniere.parts : null;
      if (s) setChat(s);
      follow.current = true;
      setItems((cur) =>
        cur.map((i) => {
          if (i.kind !== "message" || !i.live) return i;
          let parts = relue ?? i.parts;
          if (!textOf({ ...i, parts }) && r.text) parts = appendText(parts, r.text);
          return {
            ...i,
            parts,
            live: false,
            run: r.run,
            propositions: [...i.propositions, ...r.propositions.filter((p) => !i.propositions.some((q) => q.id === p.id))],
            task: r.tache && r.fichiers?.length ? { tache: r.tache, fichiers: r.fichiers } : undefined,
          };
        }),
      );
      setStatus(r.stopped_by ? `arrêté ${r.stopped_by === "ide" ? "dans l'IDE" : `par ${r.stopped_by}`}` : "fini");
      // Les compteurs du tour, un total par unité, le modèle nommé une fois.
      const byUnit = new Map<string, number>();
      for (const c of r.consumed ?? []) byUnit.set(c.unit, (byUnit.get(c.unit) ?? 0) + c.amount);
      setConsumed(byUnit.size ? [...byUnit].map(([u, n]) => `${u.replace(/_tokens$/, "")} ${tokens(n)}`).join(" · ") : "");
    } catch (e) {
      setItems((cur) => [...cur.filter((i) => !(i.kind === "message" && i.live)), message("error", (e as Error).message)]);
      setStatus("Erreur");
    } finally {
      setBusy(false);
      setRun(null);
      reloadRecaps();
    }
  };

  const stop = async () => {
    if (!run) return;
    setStatus("arrêt demandé…");
    try {
      await stopRun(id, run);
    } catch (e) {
      setStatus((e as Error).message);
    }
  };

  if (!backend) return <p className="text-destructive">conversation inconnue : {id}</p>;
  const task = backend.tache;
  // **La barre parle en dépôts, worktrees et branches** (Lucie, 7 novembre 2026) :
  // « Dépôts » range chaque dépôt avec sa branche, puis les worktrees de ses
  // tâches (leur branche `agent/<nom>`) ; « Mémoire » ; « Projets » pour un projet
  // déclaratif sans dépôt. Une tâche se range sous le dépôt de son parent.
  type Group = { label: string; main?: Backend; items: Backend[] };
  const byId = new Map(backendsHere.map((b) => [b.id, b]));
  const repoOf = (b: Backend) => (b.tache ? byId.get(b.tache.parent)?.repo ?? b.repo : b.repo) ?? displayName(b);
  const depots = new Map<string, Group>();
  for (const b of backendsHere.filter((b) => (b.kind ?? "code") === "code" || b.tache)) {
    const label = repoOf(b);
    const g = depots.get(label) ?? { label, items: [] };
    if (!b.tache && !g.main) g.main = b;
    g.items.push(b);
    depots.set(label, g);
  }
  for (const g of depots.values()) g.items.sort((a, b) => (a === g.main ? -1 : b === g.main ? 1 : displayName(a).localeCompare(displayName(b))));
  const solo = (kind: string): Group[] => backendsHere.filter((b) => b.kind === kind && !b.tache).map((b) => ({ label: displayName(b), main: b, items: [b] }));
  const sections = [
    { title: "Dépôts", groups: [...depots.values()] },
    { title: "Mémoire", groups: solo("memory") },
    { title: "Projets", groups: solo("project") },
  ];
  const roles = [...new Set(roleRuns.map((r) => r.role))].sort((a, b) => (a === "archiviste" ? -1 : b === "archiviste" ? 1 : a.localeCompare(b)));
  return (
    <div className="chat grid md:grid-cols-[300px_1fr] flex-1 min-h-0">
      <aside className="hidden md:flex flex-col gap-1 border-r bg-sidebar text-sidebar-foreground p-4 min-h-0 overflow-auto">
        <div className="brand font-semibold tracking-tight text-lg">
          rag3weaver <span className="block text-[11px] tracking-widest text-muted-foreground uppercase">atelier local</span>
        </div>
        <nav id="sessions" className="flex flex-col">
          {sections.map(({ title, groups }) =>
            groups.length ? (
              <section key={title} className="mt-4" data-section={title}>
                <h2 className="text-[11px] uppercase tracking-wider text-muted-foreground mb-1">{title}</h2>
                {groups.map((g) => (
                  <div key={g.label} className="depot" data-depot={g.label}>
                    {g.main ? null : <div className="text-sm font-medium px-2 truncate">{g.label}</div>}
                    {g.items.map((b) => (
                      <div key={b.id} className={`conversation ${g.main && b.id === g.main.id ? "" : "pl-3"}`} data-backend={b.id} data-kind={b.kind ?? ""}>
                        <a href={`/ide/code/${b.id}/chat`} className={`block text-sm truncate rounded px-2 py-1 hover:bg-sidebar-accent ${b.id === id ? "bg-sidebar-accent active" : ""}`} data-backend={b.id}>
                          {b.tache ? <GitBranch className="inline size-3.5 mr-1" aria-hidden /> : <MessageSquare className="inline size-3.5 mr-1" aria-hidden />}
                          {displayName(b)}
                        </a>
                        <Gauge context={recaps[b.id]?.context} model={recaps[b.id]?.model} counters={b.id === id ? consumed : undefined} />
                        {b.kind === "memory" ? null : <Writes writes={recaps[b.id]?.writes} why={recaps[b.id]?.writesWhy} open={b.id === id} />}
                      </div>
                    ))}
                  </div>
                ))}
              </section>
            ) : null,
          )}
        </nav>
        {roles.length ? (
          <section id="runs-a-role" className="mt-3">
            <h2 className="text-[11px] uppercase tracking-wider text-muted-foreground">Tâches de fond</h2>
            {roles.map((role) => (
              <div key={role} className="mt-1" data-role={role}>
                <div className="text-xs font-semibold">{role}</div>
                {roleRuns
                  .filter((r) => r.role === role)
                  .map((r) => (
                    <div key={r.run} className="run-a-role text-[11px] text-muted-foreground flex justify-between gap-2" data-run={r.run} data-state={r.state}>
                      <span className="truncate">{r.what}</span>
                      <span>
                        {r.state}
                        {r.at ? ` · ${new Date(r.at).toLocaleTimeString()}` : ""}
                      </span>
                    </div>
                  ))}
              </div>
            ))}
          </section>
        ) : null}
        <p className="hint text-xs text-muted-foreground mt-auto pt-3">
          {task ? "Une tâche : l'agent écrit sur sa branche ; la relecture et la fusion sont dans les tâches." : "L'agent propose ; vous appliquez ou rejetez dans les propositions."}
        </p>
      </aside>
      <main className="flex flex-col min-h-0 min-w-0">
        <header className="flex items-center justify-between gap-3 px-6 py-3 border-b">
          <div className="min-w-0">
            <h1 id="name" className="text-base font-semibold truncate">
              {task ? `Chat de la tâche ${task.nom} · ${task.branche}` : `Chat · ${displayName(backend)}`}
            </h1>
            <p id="model" className="text-xs text-muted-foreground">{chat ? (chat.on ? `modèle : ${chat.model}` : `chat éteint : ${chat.why}`) : "Connexion…"}</p>
          </div>
          <nav className="flex items-center gap-3 text-xs">
            {task ? (
              <a href={`/ide/code/${task.parent}/taches/${encodeURIComponent(task.nom)}`} className="inline-flex items-center gap-1 hover:underline" data-relecture>
                <FileDiff className="size-3.5" /> relecture
              </a>
            ) : null}
            <a href={`/ide/code/${id}/bilans`} className="inline-flex items-center gap-1 hover:underline" data-bilans>
              <Scale className="size-3.5" /> bilans
            </a>
            <span id="status" role="status" className="text-primary">
              {status}
            </span>
          </nav>
        </header>
        {chat?.orphans?.length ? (
          <p className="alerte text-destructive text-xs px-6 py-2" id="chat-orphelines">
            {chat.orphans.length} session(s) orpheline(s), jamais rouvertes : {chat.orphans.map((o) => o.pourquoi).join(" ; ")}
          </p>
        ) : null}
        <section
          id="messages"
          ref={messages}
          aria-label="Conversation"
          aria-live="polite"
          className="flex-1 min-h-0 overflow-auto px-6 py-6"
          onScroll={(e) => {
            const m = e.currentTarget;
            stuck.current = m.scrollHeight - m.scrollTop - m.clientHeight < 40;
          }}
        >
          {items.length ? null : <p className="empty text-muted-foreground mt-[12vh]">Une question, une recherche, une idée à explorer.</p>}
          {items.map((it, k) =>
            it.kind === "info" ? (
              <p key={k} className="text-xs text-muted-foreground">{it.text}</p>
            ) : (
              <article key={k} className={`message ${it.role} ${it.live ? "en-cours" : ""} mb-7`} data-role={it.role} data-run={it.run}>
                <div className={`role text-xs font-semibold mb-1.5 ${it.role === "user" ? "text-blue-400" : it.role === "error" ? "text-destructive" : "text-primary"}`}>
                  {it.role === "user" ? "Vous" : it.role === "error" ? "Erreur" : "Agent"}
                </div>
                {it.live && !it.parts.length ? <div className="body whitespace-pre-wrap break-words max-w-[100ch]" /> : null}
                {it.parts.map((p, n) =>
                  p.kind === "text" ? (
                    it.role === "assistant" && !it.live ? (
                      <div key={n} className="body markdown break-words max-w-[100ch]" dangerouslySetInnerHTML={{ __html: renderMarkdown(p.text) }} />
                    ) : (
                      <div key={n} className="body whitespace-pre-wrap break-words max-w-[100ch]">{p.text}</div>
                    )
                  ) : (
                    <details key={p.id} className="outil border rounded-lg px-3.5 py-2.5 my-3 text-[13px] text-muted-foreground max-w-[100ch]" data-outil={p.name} data-appel={p.id} data-ok={p.ok === undefined ? undefined : String(p.ok)}>
                      <summary className="cursor-pointer text-foreground/80">
                        {p.name}
                        {p.result === undefined ? "…" : p.ok === false ? " : refusé" : ""}
                        <span className="heure text-[11px] text-muted-foreground ml-2">{when(p)}</span>
                      </summary>
                      <div className="text-xs mt-2">
                        <div className="arguments font-mono whitespace-pre-wrap break-words">{p.args}</div>
                        {p.result !== undefined ? (
                          <pre className="resultat whitespace-pre-wrap break-words max-h-[420px] overflow-auto mt-2 border-t pt-2">
                            {p.result || "(aucun résultat)"}
                            {p.truncated ? "\n… (extrait ; le résultat entier se relit à la fin du tour)" : ""}
                          </pre>
                        ) : null}
                      </div>
                    </details>
                  ),
                )}
                {it.propositions.map((p) => (
                  <p key={p.id} className="proposition-du-tour text-sm">
                    <a href={`/ide/code/${id}/propositions/${p.id}`} data-proposition={p.id} className="underline">
                      proposition #{p.id}
                    </a>{" "}
                    — {(p.paths ?? []).join(", ")}
                    {p.accepted === false ? " (refusée à la validation)" : ""}
                  </p>
                ))}
                {it.task ? (
                  <p className="tache-du-tour text-sm">
                    {it.task.fichiers.join(", ")} — sur la branche {it.task.tache.branche} ·{" "}
                    <a href={`/ide/code/${it.task.tache.parent}/taches/${encodeURIComponent(it.task.tache.nom)}`} data-tache={it.task.tache.nom} className="underline">
                      relire la tâche
                    </a>
                  </p>
                ) : null}
              </article>
            ),
          )}
        </section>
        <form
          id="composer"
          className="w-full px-6 pb-4 pt-2 border-t"
          onSubmit={(e) => {
            e.preventDefault();
            send();
          }}
        >
          <label htmlFor="message" className="block text-xs text-muted-foreground mb-1">
            Votre message
          </label>
          <div className="flex items-end gap-2.5">
            <textarea
              id="message"
              rows={3}
              required
              disabled={!chat?.on}
              placeholder="Que voulez-vous explorer ?"
              ref={input}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  send();
                }
              }}
              className="flex-1 min-w-0 min-h-[90px] max-h-[250px] resize-y rounded-xl border bg-input/30 px-3.5 py-3 focus:outline focus:outline-ring"
            />
            <div className="actions flex flex-col gap-2 shrink-0">
              <Button id="stop" type="button" variant="outline" size="sm" hidden={!run} onClick={stop}>
                <Square className="size-3.5" /> Arrêter
              </Button>
              <Button id="send" type="submit" size="sm" disabled={busy || !chat?.on}>
                Envoyer <SendHorizontal className="size-3.5" />
              </Button>
            </div>
          </div>
          <small id="aide" className="block mt-1.5 text-[11px] text-muted-foreground whitespace-nowrap">Entrée pour envoyer · Maj + Entrée pour une nouvelle ligne</small>
        </form>
      </main>
    </div>
  );
}
