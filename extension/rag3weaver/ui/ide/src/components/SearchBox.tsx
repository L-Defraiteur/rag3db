import { useState } from "react";
import { Search as SearchIcon, X } from "lucide-react";
import { Button } from "@/components/ui/button";

// **La saisie de la recherche** : disponible partout où un backend est connu (l'accueil,
// l'arbre, la vue recherche), sans avoir exploré de dossier. Le dossier courant n'est
// qu'un filtre facultatif : prérempli depuis l'arbre, il part en `dossier` dans l'URL
// (passé à `search_code` par `scope`), et s'efface d'un clic. Sans dossier, tout l'index.
// Repères des témoins : `#chercher`, `input[name=q]`, `#chercher-dossier` (la pastille) et
// son `button` (l'effacement).
//
// Avant (l'ancienne page), le formulaire était caché et seul le fil d'Ariane d'un arbre ou
// d'un fichier le révélait : il fallait l'id d'un backend et seul le fil le connaissait.

export function searchLink(id: string, q: string, dir?: string | null): string {
  const p = new URLSearchParams({ q });
  if (dir) p.set("dossier", dir);
  return `/ide/code/${id}/recherche?${p.toString()}`;
}

/// Un préfixe de dossier normalisé (`src/x/`), ou null pour la racine.
export function normalizeDir(dir: string | null | undefined): string | null {
  const d = (dir ?? "").replace(/^\/+/, "");
  if (!d) return null;
  return d.endsWith("/") ? d : d + "/";
}

export function SearchBox({
  id,
  q = "",
  dir,
  onClearDir,
  domId = "chercher",
}: {
  id: string;
  q?: string;
  dir?: string | null;
  /// Effacer le filtre : par défaut, la saisie l'oublie pour la prochaine recherche ; la
  /// vue recherche relance la même question sans dossier.
  onClearDir?: () => void;
  /// L'id du formulaire : `chercher` quand il est seul dans la page ; l'accueil en montre
  /// un par backend.
  domId?: string;
}) {
  const [scope, setScope] = useState<string | null>(normalizeDir(dir));
  return (
    <form
      id={domId}
      className="flex flex-wrap items-center gap-2 mb-4"
      onSubmit={(e) => {
        e.preventDefault();
        const v = String(new FormData(e.currentTarget).get("q") ?? "").trim();
        if (v) location.href = searchLink(id, v, scope);
      }}
    >
      <input name="q" type="search" defaultValue={q} placeholder="chercher dans le code" aria-label="chercher dans le code" className="flex-1 min-w-48 rounded-md border bg-input/30 px-3 py-1.5 text-sm" />
      {scope ? (
        <span id={`${domId}-dossier`} data-dossier={scope} className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs text-muted-foreground">
          dans <span className="font-mono text-foreground">{scope}</span>
          <button
            type="button"
            aria-label={`chercher partout, pas seulement dans ${scope}`}
            title="chercher dans tout l'index"
            className="rounded hover:bg-muted p-0.5"
            onClick={() => {
              setScope(null);
              onClearDir?.();
            }}
          >
            <X className="size-3" />
          </button>
        </span>
      ) : null}
      <Button type="submit" size="sm" variant="outline">
        <SearchIcon className="size-3.5" /> Chercher
      </Button>
    </form>
  );
}
