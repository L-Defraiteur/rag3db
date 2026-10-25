import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Moon, Sun, Radio } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { fetchState, openFlux, type FluxStatus, type HostState } from "@/lib/host";
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
  useEffect(() => {
    const reload = () => fetchState().then(setState).catch((e: Error) => setError(e.message));
    reload();
    return openFlux(
      () => {},
      (s: FluxStatus) => {
        setFlux(s);
        if (s === "ouvert") reload();
      },
    );
  }, []);

  const dark = effectiveTheme(theme) === "dark";
  return (
    <div className="min-h-dvh flex flex-col bg-background text-foreground">
      <header className="flex items-center gap-3 px-4 py-2 border-b" id="entete">
        <a href="/ide/" className="font-semibold tracking-tight">
          rag3weaver
        </a>
        <span className="text-xs text-muted-foreground">IDE</span>
        <span className="ml-auto flex items-center gap-1 text-xs text-muted-foreground" id="lien" data-flux={flux}>
          <Radio className="size-3" aria-hidden /> flux : {flux}
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
          <Home state={state} />
        ) : (
          <p className="text-muted-foreground">
            <Badge variant="outline">…</Badge> l'état de l'hôte se lit
          </p>
        )}
      </main>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
