import { useEffect, useRef, useState } from "react";
import { Square, SendHorizontal, MessageSquare, FileDiff, Scale } from "lucide-react";
import { Button } from "@/components/ui/button";
import { fetchChat, onFlux, sendChat, shortName, stopRun, type Backend, type ChatTurn, type FluxEvent, type HostState, type TurnResult } from "@/lib/host";

// **Le chat de la tâche et de l'atelier** (page 08, vue 2) : la forme de ui/chat —
// la colonne des conversations, l'en-tête nom / modèle / état, le fil, la saisie
// en bas (Entrée envoie, Maj + Entrée saute une ligne), la reprise après
// rechargement — branchée sur le flux du chat de l'hôte ; plus les outils
// dépliés sous la réponse pendant le flux, le lien vers la relecture quand un
// tour a écrit, et le lien « bilans ». Un seul composant pour les deux chats.

type ToolBlock = { id: string; name: string; args: string; result?: string; ok?: boolean };
type Item =
  | { kind: "message"; role: "user" | "assistant" | "error"; text: string; run?: string; live?: boolean; tools: ToolBlock[]; propositions: TurnResult["propositions"]; task?: { tache: NonNullable<TurnResult["tache"]>; fichiers: string[] } }
  | { kind: "info"; text: string };

function fromTurns(turns: ChatTurn[]): Item[] {
  const items: Item[] = [];
  const byCall = new Map<string, ToolBlock>();
  for (const t of turns) {
    if (t.role === "tool") {
      const b = byCall.get(t.tool_call_id ?? "");
      if (b) b.result = t.content ?? "";
      continue;
    }
    if (t.role !== "user" && t.role !== "assistant") continue;
    const tools: ToolBlock[] = (t.tool_calls ?? []).map((c) => ({ id: c.id, name: c.name, args: c.arguments }));
    for (const b of tools) byCall.set(b.id, b);
    if (!t.content && !tools.length) continue;
    items.push({ kind: "message", role: t.role, text: t.content ?? "", tools, propositions: [] });
  }
  return items;
}

export function Chat({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [chat, setChat] = useState<Awaited<ReturnType<typeof fetchChat>> | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [status, setStatus] = useState("Prêt");
  const [busy, setBusy] = useState(false);
  const [run, setRun] = useState<string | null>(null);
  const [consumed, setConsumed] = useState("");
  // La saisie n'est pas contrôlée par React : un témoin (ou une saisie
  // automatique du navigateur) pose la valeur par le DOM, et `send` la lit là.
  const input = useRef<HTMLTextAreaElement>(null);
  const messages = useRef<HTMLElement>(null);
  const runRef = useRef<string | null>(null);
  runRef.current = run;

  // L'état du chat, à l'ouverture et à chaque réouverture du flux : un tour parti
  // avant (page rechargée, flux coupé puis repris) a publié son run.
  const reload = async () => {
    const s = await fetchChat(id);
    setChat(s);
    setItems((cur) => (cur.some((i) => i.kind === "message" && i.live) ? cur : fromTurns(s.turns)));
    if (s.run) {
      setRun(s.run);
      setBusy(true);
      setStatus("l'agent travaille…");
      setItems((cur) => (cur.some((i) => i.kind === "message" && i.live) ? cur.map((i) => (i.kind === "message" && i.live ? { ...i, run: s.run! } : i)) : [...cur, { kind: "message", role: "assistant", text: "", run: s.run!, live: true, tools: [], propositions: [] }]));
    }
  };
  useEffect(() => {
    reload().catch((e: Error) => setStatus(e.message));
    const w = window as unknown as { __ide?: { flux?: EventSource; relireLeChat?: () => Promise<void> } };
    if (w.__ide) w.__ide.relireLeChat = reload;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  // Le flux : les jetons, les appels d'outil, les propositions du run en cours.
  useEffect(() => {
    const live = (f: (m: Extract<Item, { kind: "message" }>) => Extract<Item, { kind: "message" }>) =>
      setItems((cur) => cur.map((i) => (i.kind === "message" && i.live ? f(i) : i)));
    return onFlux((d: FluxEvent) => {
      const ev = d.evenement ?? {};
      // Le flux rouvert (page à peine chargée, flux coupé puis repris) : l'état
      // du chat se relit, avant tout filtre — ce signal n'a pas de backend.
      if (d.sujet === "flux" && ev.kind === "ouvert") {
        reload().catch(() => {});
        return;
      }
      if (!backend || d.backend !== backend.manifest) return;
      if (ev.kind === "RunStarted" && ev.run_kind === "agent" && ev.name === `chat:${id}`) {
        setRun(String(ev.run));
        setBusy(true);
        live((m) => ({ ...m, run: String(ev.run) }));
        return;
      }
      if (ev.run !== undefined && runRef.current && ev.run !== runRef.current) return;
      if (ev.kind === "Token") live((m) => ({ ...m, text: m.text + String(ev.text ?? "") }));
      else if (ev.kind === "ToolCallStarted") {
        setStatus(String(ev.tool));
        live((m) => ({ ...m, tools: [...m.tools, { id: String(ev.call_id), name: String(ev.tool), args: String(ev.arguments ?? "") }] }));
      } else if (ev.kind === "ToolCallFinished") {
        live((m) => ({ ...m, tools: m.tools.map((t) => (t.id === String(ev.call_id) ? { ...t, ok: ev.ok !== false, result: ev.ok === false ? `refusé (${ev.error_kind ?? "erreur"})` : t.result ?? "fait" } : t)) }));
      } else if (ev.kind === "PropositionChanged" && ev.change === "proposed") {
        live((m) => (m.propositions.some((p) => p.id === ev.id) ? m : { ...m, propositions: [...m.propositions, { id: Number(ev.id), paths: ev.paths as string[] | undefined, accepted: ev.accepted as boolean | undefined }] }));
      } else if (ev.kind === "RunFinished" && ev.run === runRef.current) {
        setRun(null);
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, backend?.manifest]);

  useEffect(() => {
    const m = messages.current;
    if (m) m.scrollTop = m.scrollHeight;
  }, [items]);

  const send = async () => {
    const message = input.current?.value.trim() ?? "";
    if (!message || busy || !chat?.on) return;
    if (input.current) input.current.value = "";
    setBusy(true);
    setStatus("l'agent travaille…");
    setConsumed("");
    setItems((cur) => [...cur, { kind: "message", role: "user", text: message, tools: [], propositions: [] }, { kind: "message", role: "assistant", text: "", live: true, tools: [], propositions: [] }]);
    try {
      const r = await sendChat(id, message);
      setItems((cur) =>
        cur.map((i) =>
          i.kind === "message" && i.live
            ? {
                ...i,
                live: false,
                run: r.run,
                text: r.text || i.text,
                propositions: [...i.propositions, ...r.propositions.filter((p) => !i.propositions.some((q) => q.id === p.id))],
                task: r.tache && r.fichiers?.length ? { tache: r.tache, fichiers: r.fichiers } : undefined,
              }
            : i,
        ),
      );
      setStatus(r.stopped_by ? `arrêté ${r.stopped_by === "ide" ? "dans l'IDE" : `par ${r.stopped_by}`}` : "fini");
      setConsumed(r.consumed?.length ? `consommé : ${r.consumed.map((c) => `${c.slug} ${c.amount} ${c.unit}`).join(", ")}` : "");
    } catch (e) {
      setItems((cur) => [...cur.filter((i) => !(i.kind === "message" && i.live)), { kind: "message", role: "error", text: (e as Error).message, tools: [], propositions: [] }]);
      setStatus("Erreur");
    } finally {
      setBusy(false);
      setRun(null);
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

  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const task = backend.tache;
  const siblings: Backend[] = state.backends.filter((b) => b.id === id || (task ? b.id === task.parent || b.tache?.parent === task.parent : b.tache?.parent === id));
  return (
    <div className="chat grid md:grid-cols-[260px_1fr] h-[calc(100dvh-3rem)] -mx-4 -my-6">
      <aside className="hidden md:flex flex-col gap-2 border-r bg-sidebar text-sidebar-foreground p-4 overflow-auto">
        <div className="brand font-semibold tracking-tight text-lg">
          rag3weaver <span className="block text-[11px] tracking-widest text-muted-foreground uppercase">atelier local</span>
        </div>
        <h2 className="text-[11px] uppercase tracking-wider text-muted-foreground mt-4">Conversations</h2>
        <nav id="sessions" className="flex flex-col gap-1">
          {siblings.map((b) => (
            <a key={b.id} href={`/ide/code/${b.id}/chat`} className={`text-sm truncate rounded px-2 py-1 hover:bg-sidebar-accent ${b.id === id ? "bg-sidebar-accent active" : ""}`} data-backend={b.id}>
              <MessageSquare className="inline size-3.5 mr-1" aria-hidden />
              {b.tache ? `tâche ${b.tache.nom}` : shortName(b.manifest)}
            </a>
          ))}
        </nav>
        <p className="hint text-xs text-muted-foreground mt-auto">
          {task ? "Une tâche : l'agent écrit sur sa branche ; la relecture et la fusion sont dans les tâches." : "L'agent propose ; vous appliquez ou rejetez dans les propositions."}
        </p>
      </aside>
      <main className="flex flex-col min-h-0 min-w-0">
        <header className="flex items-center justify-between gap-3 px-6 py-3 border-b">
          <div className="min-w-0">
            <h1 id="name" className="text-base font-semibold truncate">
              {task ? `Chat de la tâche ${task.nom}` : `Chat de l'atelier · ${shortName(backend.manifest)}`}
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
        <section id="messages" ref={messages} aria-label="Conversation" aria-live="polite" className="flex-1 min-h-0 overflow-auto px-6 py-6 scroll-smooth">
          <div className="max-w-[850px] mx-auto">
            {items.length ? null : <p className="empty text-muted-foreground mt-[12vh]">Une question, une recherche, une idée à explorer.</p>}
            {items.map((it, k) =>
              it.kind === "info" ? (
                <p key={k} className="text-xs text-muted-foreground">{it.text}</p>
              ) : (
                <article key={k} className={`message ${it.role} ${it.live ? "en-cours" : ""} mb-7`} data-role={it.role} data-run={it.run}>
                  <div className={`role text-xs font-semibold mb-1.5 ${it.role === "user" ? "text-blue-400" : it.role === "error" ? "text-destructive" : "text-primary"}`}>
                    {it.role === "user" ? "Vous" : it.role === "error" ? "Erreur" : "Agent"}
                  </div>
                  <div className="body whitespace-pre-wrap break-words">{it.text}</div>
                  {it.tools.map((t) => (
                    <details key={t.id} className="outil border rounded-lg px-3.5 py-2.5 my-3 text-[13px] text-muted-foreground" data-outil={t.name} data-appel={t.id} data-ok={t.ok === undefined ? undefined : String(t.ok)}>
                      <summary className="cursor-pointer text-foreground/80">
                        {t.name}
                        {t.result === undefined ? "…" : t.ok === false ? ` : ${t.result}` : ""}
                      </summary>
                      <pre className="whitespace-pre-wrap break-words max-h-[340px] overflow-auto text-xs mt-2">{t.args}{t.result !== undefined && t.ok !== false ? `\n\n${t.result}` : ""}</pre>
                    </details>
                  ))}
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
          </div>
        </section>
        <form
          id="composer"
          className="mx-auto w-full max-w-[850px] px-6 pb-5 pt-2"
          onSubmit={(e) => {
            e.preventDefault();
            send();
          }}
        >
          <label htmlFor="message" className="block text-xs text-muted-foreground mb-1">
            Votre message
          </label>
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
            className="w-full min-h-[90px] max-h-[250px] resize-y rounded-xl border bg-input/30 px-3.5 py-3 focus:outline focus:outline-ring"
          />
          <div className="actions flex items-center gap-2.5 mt-2.5">
            <small className="flex-1 text-[11px] text-muted-foreground">Entrée pour envoyer · Maj + Entrée pour une nouvelle ligne</small>
            <span id="chat-conso" className="text-[11px] text-muted-foreground">{consumed}</span>
            <Button id="stop" type="button" variant="outline" size="sm" hidden={!run} onClick={stop}>
              <Square className="size-3.5" /> Arrêter
            </Button>
            <Button id="send" type="submit" size="sm" disabled={busy || !chat?.on}>
              Envoyer <SendHorizontal className="size-3.5" />
            </Button>
          </div>
        </form>
      </main>
    </div>
  );
}
