// **Le mode sombre** : la préférence du système par défaut, et le choix de la
// personne (localStorage) quand elle a cliqué le bouton de l'en-tête. La classe
// `dark` sur <html> commande les variables CSS de shadcn.
export type Theme = "light" | "dark" | "system";
const KEY = "ide-theme";

export function storedTheme(): Theme {
  try {
    const v = localStorage.getItem(KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

export function effectiveTheme(t: Theme): "light" | "dark" {
  if (t !== "system") return t;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(t: Theme) {
  const dark = effectiveTheme(t) === "dark";
  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.dataset.theme = effectiveTheme(t);
  try {
    if (t === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, t);
  } catch {
    /* sans stockage, le choix vaut pour la page ouverte */
  }
}
