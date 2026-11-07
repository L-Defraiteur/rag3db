import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight, GitBranch, FolderGit2, Bot, Brain } from "lucide-react";
import { displayName, type ChatSession, type HostState, type Rag, type Worktree } from "@/lib/host";

// **La barre du chat** (ticket « la barre du chat ne se lit pas par session », forme
// tranchée avec Lucie) : par session (l'agent ; dessous ses dépôts, leurs worktrees, ses
// sous-sessions, sa mémoire) ou par dépôt (dépôt › worktree › les sessions qui le
// tiennent), au choix d'un menu mémorisé ; tout replié sauf le chemin actif ; un filtre
// « actifs seulement » ; les dépôts que personne ne tient en bas, « disponibles ». Le
// worktree est la feuille : c'est lui qu'on ouvre (sa conversation).

type Vue = "session" | "depot";
const VUE = "rag3weaver.ide.barre.vue";
const lireVue = (): Vue => {
  try {
    return localStorage.getItem(VUE) === "depot" ? "depot" : "session";
  } catch {
    return "session";
  }
};

const ragTexte = (r: Rag) => (r.state === "indexed" ? "rag indexé" : r.state === "indexing" ? `rag en cours${r.percent != null ? ` ${r.percent} %` : ""}` : "rag absent");
const etatTexte: Record<string, string> = { running: "en cours", waiting: "en attente", ended: "finie", error: "en erreur" };
const actif = (s: ChatSession) => s.state === "running" || s.state === "waiting" || s.repos.some((r) => r.worktrees.some((w) => w.loaded));

/// Le chemin de la session de la page jusqu'à la racine : ce qui reste déplié.
export function sessionPath(sessions: ChatSession[], id: string): ChatSession[] {
  const par = new Map(sessions.map((s) => [s.id, s]));
  const chemin: ChatSession[] = [];
  let s = par.get(id);
  while (s && !chemin.includes(s)) {
    chemin.unshift(s);
    s = s.parent ? par.get(s.parent) : undefined;
  }
  return chemin;
}

/// Le fil d'Ariane de la page : session › sous-session › worktree.
export function Breadcrumb({ state, id }: { state: HostState; id: string }) {
  const sessions = state.chat_sessions ?? [];
  const chemin = sessionPath(sessions, id);
  const ici = sessions.find((s) => s.id === id);
  const w = ici?.repos[0]?.worktrees.find((x) => x.backend === id);
  if (!chemin.length) return null;
  return (
    <nav id="fil-chat" aria-label="fil d'Ariane" className="text-xs text-muted-foreground">
      {chemin.map((s, k) => (
        <span key={s.id}>
          {k ? " › " : ""}
          <a href={`/ide/code/${s.id}/chat`} className="hover:underline">{s.name ?? s.id}</a>
        </span>
      ))}
      {ici?.repos[0] ? ` › ${ici.repos[0].repo ?? ""}` : ""}
      {w?.branch ? ` › ${w.branch}` : ""}
    </nav>
  );
}

function Repli({ titre, ouvert, children, attrs }: { titre: ReactNode; ouvert: boolean; children: ReactNode; attrs?: Record<string, string> }) {
  const [o, setO] = useState(ouvert);
  return (
    <div {...attrs}>
      <button type="button" className="repli flex items-center gap-1 text-left w-full text-sm hover:bg-sidebar-accent rounded px-1" onClick={() => setO(!o)} aria-expanded={o}>
        {o ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        {titre}
      </button>
      {o ? <div className="pl-3">{children}</div> : null}
    </div>
  );
}

function WorktreeLine({ w, id, sessions }: { w: Worktree; id: string; sessions: ChatSession[] }) {
  const tenant = w.held_by ? sessions.find((s) => s.id === w.held_by) : undefined;
  return (
    <a
      href={`/ide/code/${w.backend}/chat`}
      className={`worktree block text-xs rounded px-1 py-0.5 hover:bg-sidebar-accent ${w.backend === id ? "bg-sidebar-accent active" : ""}`}
      data-backend={w.backend}
      data-loaded={String(w.loaded)}
      data-rag={w.rag.state}
    >
      <GitBranch className="inline size-3 mr-1" aria-hidden />
      {w.branch ?? "?"} · {w.loaded ? "chargé" : "non chargé"} · {ragTexte(w.rag)}
      {tenant && w.held_by !== id ? <span className="tenu-par ml-1 rounded border px-1">tenu par {tenant.name ?? tenant.id}</span> : null}
    </a>
  );
}

function SessionNode({ s, sessions, id, ouverts, extra }: { s: ChatSession; sessions: ChatSession[]; id: string; ouverts: Set<string>; extra: (s: ChatSession) => ReactNode }) {
  const enfants = sessions.filter((x) => x.parent === s.id);
  const ouvert = ouverts.has(s.id);
  return (
    <Repli
      ouvert={ouvert}
      attrs={{ class: "session", "data-session": s.id, ...(s.state ? { "data-state": s.state } : {}) }}
      titre={
        <span className="flex items-center gap-1 min-w-0">
          <Bot className="size-3.5 shrink-0" aria-hidden />
          <span className="truncate font-medium">{s.name ?? s.id}</span>
          {s.state ? <span className="etat text-[10px] text-muted-foreground">{etatTexte[s.state]}{s.awaits_answer ? " · attend une réponse" : ""}</span> : null}
        </span>
      }
    >
      <a href={`/ide/code/${s.id}/chat`} className={`conversation block text-xs rounded px-1 py-0.5 hover:bg-sidebar-accent ${s.id === id ? "bg-sidebar-accent active" : ""}`} data-backend={s.id}>
        conversation
      </a>
      {extra(s)}
      <Repli ouvert={ouvert} attrs={{ class: "groupe-depots" }} titre={<span className="text-[11px] uppercase tracking-wider text-muted-foreground">Dépôts</span>}>
        {s.repos.map((r) => (
          <div key={r.backend} className="depot" data-depot={r.repo ?? ""}>
            <div className="text-xs font-medium">
              <FolderGit2 className="inline size-3 mr-1" aria-hidden />
              {r.repo ?? "?"}
            </div>
            {r.worktrees.map((w) => (
              <WorktreeLine key={w.backend} w={w} id={id} sessions={sessions} />
            ))}
          </div>
        ))}
      </Repli>
      {enfants.length ? (
        <Repli ouvert={ouvert} attrs={{ class: "groupe-sous-sessions" }} titre={<span className="text-[11px] uppercase tracking-wider text-muted-foreground">Sous-sessions</span>}>
          {enfants.map((e) => (
            <SessionNode key={e.id} s={e} sessions={sessions} id={id} ouverts={ouverts} extra={extra} />
          ))}
        </Repli>
      ) : null}
      <div className="memoire text-xs px-1" data-memory={s.memory.backend ?? ""} data-rag={String(s.memory.rag)}>
        <Brain className="inline size-3 mr-1" aria-hidden />
        mémoire · {s.memory.backend ? (s.memory.rag ? "rag oui" : "rag non") : "non déclarée"}
      </div>
    </Repli>
  );
}

/// La barre : le menu de vue, le filtre, l'arbre, les disponibles.
export function ChatSidebar({ state, id, extra }: { state: HostState; id: string; extra: (s: ChatSession) => ReactNode }) {
  const [vue, setVue] = useState<Vue>(lireVue());
  const [actifs, setActifs] = useState(false);
  const toutes = state.chat_sessions ?? [];
  const sessions = actifs ? toutes.filter((s) => actif(s) || s.id === id) : toutes;
  const ouverts = new Set(sessionPath(toutes, id).map((s) => s.id));
  const changer = (v: Vue) => {
    setVue(v);
    try {
      localStorage.setItem(VUE, v);
    } catch {
      /* sans stockage, le choix vaut pour la page */
    }
  };
  // La vue par dépôt : dépôt › worktree › les sessions qui le tiennent, des mêmes données.
  const parDepot = new Map<string, { worktrees: Map<string, { w: Worktree; tenants: ChatSession[] }> }>();
  for (const s of sessions)
    for (const r of s.repos)
      for (const w of r.worktrees) {
        const d = parDepot.get(r.repo ?? "?") ?? { worktrees: new Map() };
        const e = d.worktrees.get(w.backend) ?? { w, tenants: [] };
        if (!e.tenants.includes(s)) e.tenants.push(s);
        d.worktrees.set(w.backend, e);
        parDepot.set(r.repo ?? "?", d);
      }
  return (
    <div id="barre" className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <select id="vue-barre" aria-label="vue de la barre" value={vue} onChange={(e) => changer(e.target.value as Vue)} className="text-xs rounded border bg-transparent px-1 py-0.5">
          <option value="session">par session</option>
          <option value="depot">par dépôt</option>
        </select>
        <label className="text-[11px] flex items-center gap-1">
          <input id="actifs-seulement" type="checkbox" checked={actifs} onChange={(e) => setActifs(e.target.checked)} /> actifs seulement
        </label>
      </div>
      {vue === "session" ? (
        <div id="arbre-sessions" data-vue="session">
          {sessions
            .filter((s) => !s.parent || !sessions.some((p) => p.id === s.parent))
            .map((s) => (
              <SessionNode key={s.id} s={s} sessions={sessions} id={id} ouverts={ouverts} extra={extra} />
            ))}
        </div>
      ) : (
        <div id="arbre-depots" data-vue="depot">
          {[...parDepot.entries()].map(([depot, d]) => (
            <Repli key={depot} ouvert attrs={{ class: "depot", "data-depot": depot }} titre={<span className="text-sm font-medium">{depot}</span>}>
              {[...d.worktrees.values()].map(({ w, tenants }) => (
                <div key={w.backend}>
                  <WorktreeLine w={w} id={id} sessions={sessions} />
                  <div className="pl-4 text-[11px] text-muted-foreground tenants">
                    {tenants.map((t) => (
                      <a key={t.id} href={`/ide/code/${t.id}/chat`} className="tenant block hover:underline" data-session={t.id}>
                        <Bot className="inline size-3 mr-1" aria-hidden />
                        {t.name ?? t.id}
                      </a>
                    ))}
                  </div>
                </div>
              ))}
            </Repli>
          ))}
        </div>
      )}
      {(state.available_repos ?? []).length ? (
        <div id="disponibles" className="mt-3">
          <h2 className="text-[11px] uppercase tracking-wider text-muted-foreground">Disponibles</h2>
          {(state.available_repos ?? []).map((r) => (
            <a key={r.backend} href={`/ide/code/${r.backend}/chat`} className="disponible block text-xs rounded px-1 py-0.5 hover:bg-sidebar-accent" data-backend={r.backend}>
              <FolderGit2 className="inline size-3 mr-1" aria-hidden />
              {r.repo ?? displayName({ id: r.backend, manifest: "", open: r.loaded, tache: null })} · {r.branch ?? "?"} · {r.loaded ? "chargé" : "non chargé"} · {ragTexte(r.rag)}
            </a>
          ))}
        </div>
      ) : null}
    </div>
  );
}
