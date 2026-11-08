// **Le thème** : la préférence du système par défaut, et le choix de la personne
// (localStorage) quand elle a cliqué un bouton de l'en-tête. La classe `dark` sur
// <html> commande les variables CSS de shadcn ; `daemon` (Lucie, 7 et 8 novembre
// 2026 : l'IDE « comme un terminal avant-gardiste », le thème du terminal) pose en
// plus les couleurs du fichier commun `templates/themes/daemon.json`, servi par
// l'hôte, en variables `--daemon-*`.
export type Theme = "light" | "dark" | "daemon" | "system";
const KEY = "ide-theme";

export function storedTheme(): Theme {
  try {
    const v = localStorage.getItem(KEY);
    return v === "light" || v === "dark" || v === "daemon" ? v : "system";
  } catch {
    return "system";
  }
}

export function effectiveTheme(t: Theme): "light" | "dark" | "daemon" {
  if (t !== "system") return t;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/// Une animation du thème, en données (la même forme que `tty/src/theme.rs`).
export type ThemeAnimation = { forme: "cercle" | "transmutation" | "glitch"; ms: number; glyphes: string[]; coeur: string; gabarits: string[] };
/// Le fichier commun, ce que la page en lit.
export type DaemonTheme = {
  nom: string;
  couleurs: { fond: string; texte: string; halo: string[]; sourdine: string };
  police: string;
  animations: Record<string, ThemeAnimation>;
  code?: Record<string, string>;
};

let daemon: Promise<DaemonTheme> | null = null;
/// Le thème commun, lu une fois par page.
export function daemonTheme(): Promise<DaemonTheme> {
  daemon ??= fetch("/ide/theme/daemon.json").then((r) => {
    if (!r.ok) throw new Error(`/ide/theme/daemon.json : HTTP ${r.status}`);
    return r.json() as Promise<DaemonTheme>;
  });
  return daemon;
}

function poserLesCouleurs(t: DaemonTheme) {
  const s = document.documentElement.style;
  s.setProperty("--daemon-fond", t.couleurs.fond);
  s.setProperty("--daemon-texte", t.couleurs.texte);
  s.setProperty("--daemon-sourdine", t.couleurs.sourdine);
  s.setProperty("--daemon-halo-vif", t.couleurs.halo[0] ?? t.couleurs.texte);
  s.setProperty("--daemon-halo-sombre", t.couleurs.halo[t.couleurs.halo.length - 1] ?? t.couleurs.texte);
  s.setProperty("--daemon-police", t.police);
  for (const [sorte, couleur] of Object.entries(t.code ?? {})) s.setProperty(`--daemon-code-${sorte.replaceAll("_", "-")}`, couleur);
}

export function applyTheme(t: Theme) {
  const e = effectiveTheme(t);
  document.documentElement.classList.toggle("dark", e !== "light");
  document.documentElement.classList.toggle("daemon", e === "daemon");
  document.documentElement.dataset.theme = e;
  if (e === "daemon") daemonTheme().then(poserLesCouleurs).catch((err: Error) => console.warn(`[thème] ${err.message} : la page garde le sombre`));
  try {
    if (t === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, t);
  } catch {
    /* sans stockage, le choix vaut pour la page ouverte */
  }
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
