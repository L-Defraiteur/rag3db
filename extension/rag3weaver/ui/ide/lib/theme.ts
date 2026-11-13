// **Le thème** : la préférence du système par défaut, et le choix de la personne
// (localStorage) quand elle a cliqué un bouton ou le menu de l'en-tête. La classe `dark`
// sur <html> commande les variables CSS de shadcn ; un thème du dossier commun
// `templates/themes/<nom>.json` (daemon, Lucie, 7 et 8 novembre 2026 : l'IDE « comme un
// terminal avant-gardiste » ; forêt et les suivants le 13), servi par l'hôte, pose en plus
// la classe `theme-fichier` et ses couleurs en variables `--theme-*`.
/// `light`, `dark`, `system`, ou le nom d'un thème du dossier.
export type Theme = string;
const KEY = "ide-theme";

/// Le choix gardé, tel quel ; un nom de thème se vérifie contre la liste de l'hôte
/// avant d'être appliqué ([`applyTheme`]).
export function storedTheme(): Theme {
  try {
    return localStorage.getItem(KEY) || "system";
  } catch {
    return "system";
  }
}

/// Un des choix sans fichier ?
export function sansFichier(t: Theme): boolean {
  return t === "light" || t === "dark" || t === "system";
}

/// Le thème qui s'applique : clair ou sombre pour le système, sinon le choix lui-même.
export function effectiveTheme(t: Theme): string {
  if (t !== "system") return t;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/// Une animation du thème, en données (la même forme que `tty/src/theme.rs`).
export type ThemeAnimation = { forme: "cercle" | "transmutation" | "glitch"; ms: number; glyphes: string[]; coeur: string; gabarits: string[] };
/// Un fichier du dossier commun, ce que la page en lit.
export type FileTheme = {
  nom: string;
  couleurs: { fond: string; texte: string; halo: string[]; sourdine: string };
  police: string;
  animations: Record<string, ThemeAnimation>;
  code?: Record<string, string>;
};

let livres: Promise<string[]> | null = null;
/// Les noms des thèmes que l'hôte sert (`/ide/themes.json`), lus une fois par page.
export function themesLivres(): Promise<string[]> {
  livres ??= fetch("/ide/themes.json").then((r) => {
    if (!r.ok) throw new Error(`/ide/themes.json : HTTP ${r.status}`);
    return r.json() as Promise<string[]>;
  });
  return livres;
}

const fichiers = new Map<string, Promise<FileTheme>>();
/// Le thème `nom` du dossier commun, lu une fois par page.
export function fileTheme(nom: string): Promise<FileTheme> {
  let p = fichiers.get(nom);
  if (!p) {
    p = fetch(`/ide/theme/${encodeURIComponent(nom)}.json`).then((r) => {
      if (!r.ok) throw new Error(`/ide/theme/${nom}.json : HTTP ${r.status}`);
      return r.json() as Promise<FileTheme>;
    });
    fichiers.set(nom, p);
  }
  return p;
}

function poserLesCouleurs(t: FileTheme) {
  const s = document.documentElement.style;
  s.setProperty("--theme-fond", t.couleurs.fond);
  s.setProperty("--theme-texte", t.couleurs.texte);
  s.setProperty("--theme-sourdine", t.couleurs.sourdine);
  s.setProperty("--theme-halo-vif", t.couleurs.halo[0] ?? t.couleurs.texte);
  s.setProperty("--theme-halo-sombre", t.couleurs.halo[t.couleurs.halo.length - 1] ?? t.couleurs.texte);
  s.setProperty("--theme-police", t.police);
  for (const [sorte, couleur] of Object.entries(t.code ?? {})) s.setProperty(`--theme-code-${sorte.replaceAll("_", "-")}`, couleur);
}

function garder(t: Theme) {
  try {
    if (t === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, t);
  } catch {
    /* sans stockage, le choix vaut pour la page ouverte */
  }
}

function poserLaMarque(e: string, fichier: boolean) {
  const h = document.documentElement;
  h.classList.toggle("dark", e !== "light");
  h.classList.toggle("theme-fichier", fichier);
  h.dataset.theme = e;
}

/// Applique le choix `t`. Un nom de thème n'est demandé à l'hôte qu'une fois vérifié dans
/// sa liste : un nom gardé que l'hôte ne connaît plus (un thème retiré) revient au choix du
/// système, sans requête ni erreur. `retour` dit le choix finalement appliqué.
export function applyTheme(t: Theme, retour?: (applique: Theme) => void) {
  if (sansFichier(t)) {
    poserLaMarque(effectiveTheme(t), false);
    garder(t);
    retour?.(t);
    return;
  }
  themesLivres()
    .then((noms) => {
      if (!noms.includes(t)) {
        applyTheme("system", retour);
        return;
      }
      poserLaMarque(t, true);
      garder(t);
      retour?.(t);
      fileTheme(t)
        .then(poserLesCouleurs)
        .catch((err: Error) => console.warn(`[thème] ${err.message} : la page garde le sombre`));
    })
    .catch((err: Error) => {
      console.warn(`[thème] ${err.message} : la page garde le choix du système`);
      applyTheme("system", retour);
    });
}

/// **L'image `n` d'une animation** : le cercle du terminal (`tty/src/anim.rs`) —
/// les glyphes tournés de `n`, les trois premiers à gauche, les trois derniers à
/// droite ; la transmutation change le glyphe du centre ; le glitch prend les
/// glyphes dans l'ordre (la page n'a pas besoin du hasard à graine du terminal).
export function imageDe(a: ThemeAnimation, n: number): string {
  const g = a.glyphes.length ? a.glyphes : [a.coeur];
  const i = n % g.length;
  const tourne = [...g.slice(i), ...g.slice(0, i)];
  const k = Math.min(3, tourne.length);
  const gabarit = a.forme === "glitch" ? a.gabarits[n % a.gabarits.length] : a.gabarits[0];
  const vals: Record<string, string> = {
    gauche: tourne.slice(0, k).join(""),
    droite: tourne.slice(tourne.length - k).join(""),
    coeur: a.coeur,
    s: g[i],
    a: tourne[0],
    b: tourne[1 % tourne.length],
    c: tourne[2 % tourne.length],
  };
  return (gabarit ?? "{coeur}").replace(/\{(\w+)\}/g, (_, nom: string) => vals[nom] ?? "");
}
