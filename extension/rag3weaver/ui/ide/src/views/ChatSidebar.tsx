import { createContext, useContext, useState, type ReactNode } from "react";
import { avancementText, useAvancement } from "@/lib/progres";
import { Bot, Brain, ChevronDown, ChevronRight, FolderGit2, GitBranch, ListTodo } from "lucide-react";
import type { ChatSession, HostState, Rag, RunState, Space, SessionTask } from "@/lib/host";

// **La barre du chat** (ticket « la barre du chat ne se lit pas par session », modèle
// tranché par c7 avec Lucie) : par session (la session, sa conversation, son dépôt,
// ses tâches et leurs espaces, sa mémoire ; une sous-tâche sous la tâche qui l'a
// déléguée) ou par dépôt (dépôt › espace › qui le tient : la session, ou la tâche et
// sa session), au choix d'un menu mémorisé. Tout est replié sauf le chemin de la
// page ; un filtre « actifs seulement » ; un fil d'Ariane ; les dépôts que personne
// ne tient en bas, « disponibles ». Un espace (worktree, ou dossier) est la feuille.

type View = "session" | "repo";
const VIEW_KEY = "rag3weaver.ide.barre.vue";
const readView = (): View => {
  try {
    return localStorage.getItem(VIEW_KEY) === "repo" ? "repo" : "session";
  } catch {
    return "session";
  }
};

const ragText = (r: Rag) => (r.state === "indexed" ? "rag indexé" : r.state === "indexing" ? `rag en cours${r.percent != null ? ` ${r.percent} %` : ""}` : "rag absent");
const stateText: Record<RunState, string> = { running: "en cours", waiting: "en attente", ended: "finie", error: "en erreur" };
// Actif : ce qui tourne, ou attend une réponse ; « en attente » seul (un équipier
// au repos, ou jamais lancé) ne l'est pas.
const taskActive = (t: SessionTask) => t.state === "running" || !!t.awaits_answer;
const sessionActive = (s: ChatSession, tasks: SessionTask[]) => s.state === "running" || !!s.awaits_answer || tasks.some((t) => t.session === s.id && taskActive(t));
const chatLink = (backend: string) => `/ide/code/${backend}/chat`;
// Le manifeste d'un backend par son id : l'avancement des index arrive par manifeste.
const Manifestes = createContext<Map<string, string>>(new Map());

/// Ce qui tient la page : la session, et la tâche si la page en est une.
export function pagePath(state: HostState, id: string): { session?: ChatSession; tasks: SessionTask[] } {
  const tasks = state.tasks ?? [];
  const byId = new Map(tasks.map((t) => [t.id, t]));
  const chain: SessionTask[] = [];
  let t = byId.get(id);
  while (t && !chain.includes(t)) {
    chain.unshift(t);
    t = t.parent_task ? byId.get(t.parent_task) : undefined;
  }
  const sid = chain[0]?.session ?? id;
  return { session: (state.chat_sessions ?? []).find((s) => s.id === sid), tasks: chain };
}

/// Le fil d'Ariane : session › tâche(s) › dépôt · branche.
export function Breadcrumb({ state, id }: { state: HostState; id: string }) {
  const { session, tasks } = pagePath(state, id);
  if (!session) return null;
  const space = (tasks.at(-1)?.spaces ?? session.spaces)[0];
  const parts: { label: string; href?: string }[] = [
    { label: session.name ?? session.id, href: chatLink(session.id) },
    ...tasks.map((t) => ({ label: t.name, href: chatLink(t.id) })),
  ];
  if (space) parts.push({ label: [space.repo, space.branch].filter(Boolean).join(" · ") });
  return (
    <nav id="fil-chat" aria-label="fil d'Ariane" className="text-xs text-muted-foreground truncate">
      {parts.map((p, k) => (
        <span key={k} className="etape">
          {k ? " › " : ""}
          {p.href ? (
            <a href={p.href} className="hover:underline">
              {p.label}
            </a>
          ) : (
            p.label
          )}
        </span>
      ))}
    </nav>
  );
}

/// Un nœud repliable ; `head` reste visible replié (la conversation d'une session et sa jauge).
function Fold({ title, head, open, children, className, data }: { title: ReactNode; head?: ReactNode; open: boolean; children: ReactNode; className?: string; data?: Record<string, string> }) {
  const [o, setO] = useState(open);
  const attrs = Object.fromEntries(Object.entries(data ?? {}).map(([k, v]) => [`data-${k}`, v]));
  return (
    <div className={className} {...attrs} data-open={String(o)}>
      <button type="button" className="repli flex items-center gap-1 text-left w-full rounded px-1 hover:bg-sidebar-accent" onClick={() => setO(!o)} aria-expanded={o}>
        {o ? <ChevronDown className="size-3 shrink-0" /> : <ChevronRight className="size-3 shrink-0" />}
        {title}
      </button>
      {head}
      {o ? <div className="pl-3">{children}</div> : null}
    </div>
  );
}

/// L'avancement de l'index d'un espace, lu sur le flux (`IndexProgress`), s'il en court un.
function Avance({ backend }: { backend: string }) {
  const manifeste = useContext(Manifestes).get(backend);
  const { a, fige, fluxOuvert } = useAvancement(manifeste);
  if (!a) return null;
  return (
    <span className="avancement ml-1" data-pourcent={a.pourcent} data-fige={fige ?? ""} data-flux={fluxOuvert ? "ouvert" : "coupe"}>
      · {avancementText(a, fige, fluxOuvert)}
    </span>
  );
}

function SpaceLine({ s, id, heldBy }: { s: Space; id: string; heldBy?: string }) {
  return (
    <a
      href={chatLink(s.backend)}
      className={`espace block text-xs rounded px-1 py-0.5 hover:bg-sidebar-accent ${s.backend === id ? "bg-sidebar-accent active" : ""}`}
      data-backend={s.backend}
      data-repo={s.repo ?? ""}
      data-loaded={String(s.loaded)}
      data-rag={s.rag.state}
      title={s.path ?? undefined}
    >
      <GitBranch className="inline size-3 mr-1" aria-hidden />
      {s.repo ?? "sans git"} · {s.branch ?? "—"} · {s.loaded ? "chargé" : "non chargé"} · {ragText(s.rag)}
      <Avance backend={s.backend} />
      {s.proposal ? <span className="proposition ml-1">· proposition {s.proposal.state === "open" ? "ouverte" : s.proposal.state === "merged" ? "fusionnée" : "rejetée"}</span> : null}
      {heldBy ? <span className="tenu-par ml-1 rounded border px-1 text-[10px]">tenu par {heldBy}</span> : null}
    </a>
  );
}

function TaskNode({ t, tasks, id, open, sessionName }: { t: SessionTask; tasks: SessionTask[]; id: string; open: Set<string>; sessionName: string }) {
  const children = tasks.filter((x) => x.parent_task === t.id);
  return (
    <Fold
      className="tache"
      data={{ task: t.id, ...(t.state ? { state: t.state } : {}) }}
      open={open.has(t.id)}
      title={
        <span className="flex items-center gap-1 min-w-0 text-sm">
          <ListTodo className="size-3.5 shrink-0" aria-hidden />
          <span className="truncate">{t.name}</span>
          {t.state ? <span className="etat text-[10px] text-muted-foreground">{stateText[t.state]}{t.awaits_answer ? " · attend une réponse" : ""}</span> : null}
        </span>
      }
    >
      <a href={chatLink(t.id)} className={`conversation-tache block text-xs rounded px-1 py-0.5 hover:bg-sidebar-accent ${t.id === id ? "bg-sidebar-accent active" : ""}`} data-backend={t.id}>
        conversation de la tâche
      </a>
      {t.spaces.map((s, k) => (
        <SpaceLine key={k} s={s} id={id} heldBy={`${t.name} (${sessionName})`} />
      ))}
      {children.length ? (
        <div className="sous-taches">
          {children.map((c) => (
            <TaskNode key={c.id} t={c} tasks={tasks} id={id} open={open} sessionName={sessionName} />
          ))}
        </div>
      ) : null}
    </Fold>
  );
}

/// La barre : le menu de vue, le filtre, l'arbre, les disponibles. `conversation`
/// rend ce qu'une session montre de sa conversation (le lien, la jauge, les fiches).
export function ChatSidebar({ state, id, conversation }: { state: HostState; id: string; conversation: (s: ChatSession) => ReactNode }) {
  const [view, setView] = useState<View>(readView());
  const [activeOnly, setActiveOnly] = useState(false);
  const allTasks = state.tasks ?? [];
  const tasks = activeOnly ? allTasks.filter((t) => taskActive(t) || t.id === id) : allTasks;
  const here = pagePath(state, id);
  const sessions = (state.chat_sessions ?? []).filter((s) => !activeOnly || s.id === here.session?.id || sessionActive(s, allTasks));
  const name = (s?: ChatSession) => s?.name ?? s?.id ?? "?";
  const open = new Set([here.session?.id, ...here.tasks.map((t) => t.id)].filter(Boolean) as string[]);
  const choose = (v: View) => {
    setView(v);
    try {
      localStorage.setItem(VIEW_KEY, v);
    } catch {
      /* sans stockage, le choix vaut pour la page */
    }
  };
  // La vue par dépôt : chaque espace des sessions et des tâches, sous son dépôt, avec qui le tient.
  const byRepo = new Map<string, { s: Space; heldBy: string }[]>();
  const add = (s: Space, heldBy: string) => byRepo.set(s.repo ?? "sans git", [...(byRepo.get(s.repo ?? "sans git") ?? []), { s, heldBy }]);
  for (const se of sessions) for (const s of se.spaces) add(s, name(se));
  for (const t of tasks) for (const s of t.spaces) add(s, `${t.name} (${name(sessions.find((x) => x.id === t.session))})`);
  return (
    <Manifestes.Provider value={new Map((state.backends ?? []).map((b) => [b.id, b.manifest]))}>
    <div id="barre" className="flex flex-col gap-1" data-view={view}>
      <div className="flex items-center gap-2 flex-wrap">
        <select id="vue-barre" aria-label="vue de la barre" value={view} onChange={(e) => choose(e.target.value as View)} className="text-xs rounded border bg-transparent px-1 py-0.5">
          <option value="session">par session</option>
          <option value="repo">par dépôt</option>
        </select>
        <label className="text-[11px] flex items-center gap-1">
          <input id="actifs-seulement" type="checkbox" checked={activeOnly} onChange={(e) => setActiveOnly(e.target.checked)} /> actifs seulement
        </label>
      </div>
      {view === "session" ? (
        <section id="arbre-sessions" data-section="Sessions">
          {sessions.map((se) => {
            const own = tasks.filter((t) => t.session === se.id && !t.parent_task);
            return (
              <Fold
                key={se.id}
                className="session"
                data={{ session: se.id, ...(se.state ? { state: se.state } : {}) }}
                open={open.has(se.id)}
                head={conversation(se)}
                title={
                  <span className="flex items-center gap-1 min-w-0 text-sm font-medium">
                    <Bot className="size-3.5 shrink-0" aria-hidden />
                    <span className="truncate">{name(se)}</span>
                    {se.model ? <span className="modele text-[10px] text-muted-foreground font-normal truncate">{se.model}</span> : null}
                  </span>
                }
              >
                {se.spaces.map((s, k) => (
                  <SpaceLine key={k} s={s} id={id} />
                ))}
                {own.length ? (
                  <Fold className="groupe-taches" open={open.has(se.id)} title={<span className="text-[11px] uppercase tracking-wider text-muted-foreground">Tâches</span>}>
                    {own.map((t) => (
                      <TaskNode key={t.id} t={t} tasks={tasks} id={id} open={open} sessionName={name(se)} />
                    ))}
                  </Fold>
                ) : null}
                <div className="memoire text-xs px-1 text-muted-foreground" data-memory={se.memory.backend ?? ""} data-rag={String(se.memory.rag)}>
                  <Brain className="inline size-3 mr-1" aria-hidden />
                  {se.memory.backend ? <a href={chatLink(se.memory.backend)} className="hover:underline">mémoire · {se.memory.rag ? "rag oui" : "rag non"}</a> : "mémoire non déclarée"}
                </div>
              </Fold>
            );
          })}
        </section>
      ) : (
        <section id="arbre-depots" data-section="Dépôts">
          {[...byRepo.entries()].map(([repo, spaces]) => (
            <Fold key={repo} className="depot" data={{ depot: repo }} open title={<span className="flex items-center gap-1 text-sm font-medium"><FolderGit2 className="size-3.5" aria-hidden />{repo}</span>}>
              {spaces.map(({ s, heldBy }, k) => (
                <SpaceLine key={k} s={s} id={id} heldBy={heldBy} />
              ))}
            </Fold>
          ))}
        </section>
      )}
      {(state.available_repos ?? []).length ? (
        <section id="disponibles" className="mt-3" data-section="Disponibles">
          <h2 className="text-[11px] uppercase tracking-wider text-muted-foreground">Disponibles</h2>
          {(state.available_repos ?? []).map((s) => (
            <SpaceLine key={s.backend} s={s} id={id} />
          ))}
        </section>
      ) : null}
    </div>
    </Manifestes.Provider>
  );
}
