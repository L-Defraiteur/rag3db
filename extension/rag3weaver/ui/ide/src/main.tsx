import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Moon, Sun, Radio } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { dispatchFlux, fetchState, openFlux, route, type FluxStatus, type HostState } from "@/lib/host";
import { Chat } from "@/views/Chat";
import { File } from "@/views/File";
import { Search } from "@/views/Search";
import { Tasks } from "@/views/Tasks";
import { TaskReview } from "@/views/TaskReview";
import { Bilans } from "@/views/Bilans";
import { Tree } from "@/views/Tree";
import { Symbol } from "@/views/Symbol";
import { GraphView } from "@/views/Graph";
import { Dag } from "@/views/Dag";
import { Reactions } from "@/views/Reactions";
import { Propositions } from "@/views/Propositions";
import { applyTheme, effectiveTheme, storedTheme, type Theme } from "@/lib/theme";
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

  useEffect(() => applyTheme(theme), [theme]);
  const reloadState = () => fetchState().then(setState).catch((e: Error) => setError(e.message));
  useEffect(() => {
    reloadState();
    return openFlux(dispatchFlux, (s: FluxStatus) => {
      setFlux(s);
      if (s === "ouvert") {
        reloadState();
        // Les vues relisent leur état à chaque ouverture du flux (le chat : un
        // tour parti avant, à rattacher).
        dispatchFlux({ sujet: "flux", backend: "*", evenement: { kind: "ouvert" } });
      }
    });
  }, []);

  const dark = effectiveTheme(theme) === "dark";
  return (
    <div className="min-h-dvh flex flex-col bg-background text-foreground">
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
      </header>
      <main id="vue" className="flex-1 px-4 py-6 max-w-5xl w-full mx-auto">
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
