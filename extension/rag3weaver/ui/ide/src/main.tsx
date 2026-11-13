import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Moon, Sun, Radio } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { dispatchFlux, fetchState, onEtat, openFlux, route, type FluxStatus, type HostState } from "@/lib/host";
import { Chat } from "@/views/Chat";
import { File } from "@/views/File";
import { Search } from "@/views/Search";
import { Recherches } from "@/views/Recherches";
import { Tasks } from "@/views/Tasks";
import { TaskReview } from "@/views/TaskReview";
import { Bilans } from "@/views/Bilans";
import { Tree } from "@/views/Tree";
import { Symbol } from "@/views/Symbol";
import { GraphView } from "@/views/Graph";
import { Dag } from "@/views/Dag";
import { Reactions } from "@/views/Reactions";
import { Propositions } from "@/views/Propositions";
import { applyTheme, effectiveTheme, storedTheme, themesLivres, type Theme } from "@/lib/theme";
import { Home } from "@/views/Home";

// **Le cadre de l'IDE** (page 08, vue 1) : l'en-tête, le flux ouvert une fois,
// l'état de l'hôte relu à chaque ouverture du flux, et la vue que l'URL nomme.
// Les vues non portées sont servies par l'hôte depuis l'ancienne page : la
// route de l'hôte choisit, la page ne les connaît pas.
function App() {
  const [state, setState] = useState<HostState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [flux, setFlux] = useState<FluxStatus>("fermé");
  const [theme, setTheme] = useState<Theme>(storedTheme());
  const [livres, setLivres] = useState<string[]>([]);

  // Un nom que l'hôte ne connaît pas revient au choix du système : l'état le suit.
  useEffect(() => applyTheme(theme, (applique) => applique !== theme && setTheme(applique)), [theme]);
  useEffect(() => {
    themesLivres()
      .then(setLivres)
      .catch((e: Error) => console.warn(`[thème] ${e.message}`));
  }, []);
  const reloadState = () => fetchState().then(setState).catch((e: Error) => setError(e.message));
  // Un worktree attaché par l'atelier (`RepoAttached`), puis la fin de son premier
  // index (`RepoIndexed`) : l'hôte pousse son état, la barre le montre sous « Dépôts »,
  // puis son rag indexé, sans recharger la page ni relire l'hôte.
  useEffect(() => onEtat<HostState>("hote", "*", (s) => setState(s)), []);
  useEffect(() => {
    reloadState();
    return openFlux(dispatchFlux, (s: FluxStatus) => {
      setFlux(s);
      // La coupure se dit aussi aux vues (l'avancement des index : « flux coupé »).
      if (s !== "ouvert") dispatchFlux({ sujet: "flux", backend: "*", evenement: { kind: "coupe" } });
      if (s === "ouvert") {
        reloadState();
        // Les vues relisent leur état à chaque ouverture du flux (le chat : un
        // tour parti avant, à rattacher).
        dispatchFlux({ sujet: "flux", backend: "*", evenement: { kind: "ouvert" } });
      }
    });
  }, []);

  const dark = effectiveTheme(theme) !== "light";
  const fichier = livres.includes(theme) ? theme : "";
  const fullWindow = route().view === "chat";
  return (
    // Le chat prend toute la fenêtre : ses barres aux bords, un seul défilement
    // (sa liste) ; les autres vues gardent leur largeur de lecture et la page
    // défile (ticket « le chat a une largeur fixe sur un écran large »).
    <div className={`${fullWindow ? "h-dvh overflow-hidden" : "min-h-dvh"} flex flex-col bg-background text-foreground`}>
      <header className="flex items-center gap-3 px-4 py-2 border-b" id="entete">
        <a href="/ide/" className="font-semibold tracking-tight">
          rag3weaver
        </a>
        <span className="text-xs text-muted-foreground">IDE</span>
        <span className="ml-auto flex items-center gap-1 text-xs text-muted-foreground">
          <Radio className="size-3" aria-hidden />
          {/* Le texte exact « flux : ouvert » : les témoins le lisent à l'égal. */}
          <span id="lien" data-flux={flux}>{`flux : ${flux}`}</span>
        </span>
        <Button
          variant="ghost"
          size="icon"
          id="theme"
          aria-label={dark ? "passer en clair" : "passer en sombre"}
          title={dark ? "passer en clair" : "passer en sombre"}
          onClick={() => setTheme(dark ? "light" : "dark")}
        >
          {dark ? <Sun className="size-4" /> : <Moon className="size-4" />}
        </Button>
        {/* Les thèmes du terminal (templates/themes/<nom>.json), ceux que l'hôte sert. */}
        <select
          id="theme-fichier"
          aria-label="thème"
          title="thème"
          className="h-8 rounded-md border bg-background px-2 text-xs text-foreground"
          value={fichier}
          onChange={(e) => setTheme(e.target.value || "system")}
        >
          <option value="">thème…</option>
          {livres.map((nom) => (
            <option key={nom} value={nom}>
              {nom}
            </option>
          ))}
        </select>
      </header>
      <main id="vue" className={fullWindow ? "flex-1 min-h-0 w-full flex flex-col" : "flex-1 px-4 py-6 max-w-5xl w-full mx-auto"}>
        {error ? (
          <p className="text-destructive" role="alert">
            {error}
          </p>
        ) : state ? (
          <View state={state} reload={reloadState} />
        ) : (
          <p className="text-muted-foreground">
            <Badge variant="outline">…</Badge> l'état de l'hôte se lit
          </p>
        )}
      </main>
    </div>
  );
}

// La vue que l'URL nomme ; une vue non portée n'arrive pas ici : l'hôte la sert
// depuis l'ancienne page.
function View({ state, reload }: { state: HostState; reload: () => Promise<unknown> }) {
  const r = route();
  if (r.view === "taches" && r.id) return r.rest ? <TaskReview id={r.id} name={r.rest} state={state} /> : <Tasks id={r.id} state={state} onChanged={reload} />;
  if (r.view === "chat" && r.id) return <Chat id={r.id} state={state} />;
  if (r.view === "fichier" && r.id) return <File id={r.id} path={r.rest ?? ""} state={state} />;
  if (r.view === "recherche" && r.id) return <Search id={r.id} state={state} />;
  if (r.view === "recherches" && r.id) return <Recherches id={r.id} state={state} />;
  if (r.view === "bilans" && r.id) return <Bilans id={r.id} state={state} />;
  if (r.view === "arbre" && r.id) return <Tree id={r.id} dir={r.rest ?? ""} state={state} />;
  if (r.view === "symbole" && r.id) return <Symbol id={r.id} name={r.rest ?? ""} state={state} />;
  if (r.view === "graphe" && r.id) return <GraphView id={r.id} name={r.rest ?? ""} state={state} />;
  if (r.view === "dag" && r.id) return <Dag id={r.id} rest={r.rest ?? ""} state={state} />;
  if (r.view === "reactions" && r.id) return <Reactions id={r.id} state={state} />;
  if (r.view === "propositions" && r.id) return <Propositions id={r.id} rest={r.rest ?? ""} state={state} />;
  return <Home state={state} />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
