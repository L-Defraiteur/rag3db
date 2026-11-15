#!/usr/bin/env python3
"""**Les thèmes dérivés de leur palette** (lot tty-12 de la session tty, 15 novembre 2026).

Lucie : « avoir vraiment des palettes à chaque fois bien strictement définies, plusieurs couleurs nommées,
et ensuite changer de thème les change juste, ainsi que la coloration syntaxique : on met un offset dessus
en originant la couleur la plus proche de celle qu'on veut atteindre ». Elle a choisi OKLCH, et la
conversion des cinq thèmes au plus près de leurs couleurs.

Un thème (`templates/themes/<nom>.json`) porte :

- `palette` : ses couleurs nommées, chacune `{"oklch": [L, C, h], "role": …}`. La forme OKLCH est
  canonique : une couleur donnée en hex (`{"hex": "#ff3333", "role": …}`) se convertit une fois, et ce
  script réécrit sa forme OKLCH. Les rôles : `fond`, `neutre`, `accent`, `halo`, `halo2`, `teinte`.
- `mode` : `sombre` ou `clair`.
- `roles` (facultatif) : une surcharge des rôles communs, prioritaire.

Les rôles communs vivent dans `templates/theme-roles.json`, hors de `templates/themes/` : l'hôte prend
tout `*.json` de ce dossier pour un thème. Pour chaque mode, ils disent quelle couleur de la palette
prend chaque rôle de `couleurs`, et comment se dérive chaque sorte de `code` :

- une sorte part d'une couleur de palette : nommée par son rôle (`"depuis": "accent"`), ou la plus
  proche en teinte d'une teinte visée (`"teinte": 55`, en degrés OKLCH), parmi les couleurs chromatiques ;
- puis elle s'en décale : la teinte va vers la visée d'au plus `DECALAGE` (une part de l'écart, bornée),
  la chroma est multipliée par `chroma`, la clarté prend la valeur `clarte` du mode ;
- enfin le contraste. **WCAG 2, en luminance relative sRGB** : au moins 4,5 contre le fond, ou 3 pour
  ce qui accompagne (commentaire, ponctuation, cadre, sourdine). La clarté s'éloigne du fond, d'un
  centième à la fois, jusqu'au seuil. Si aucune clarté ne l'atteint, le script s'arrête et le dit : il
  ne rend jamais une couleur fausse.

**La gamme** : une couleur OKLCH hors de sRGB perd de la chroma, à teinte et clarté constantes, jusqu'à y
rentrer. Jamais de coupe canal par canal.

Les champs dérivés (`couleurs.fond/texte/halo/sourdine`, `code.<sorte>`) sont écrits dans le JSON du
thème, que le terminal et l'IDE lisent tels quels. La clé `derive` dit qu'ils ne s'éditent pas à la main.

Usage :
    scripts/themes_palettes.py            réécrit chaque thème (palette canonique, champs dérivés)
    scripts/themes_palettes.py --check    rougit si un thème diffère de ce que le script écrirait
    scripts/themes_palettes.py --essais   les essais du script lui-même
"""
import json
import math
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent.parent
THEMES = ICI / "templates" / "themes"
ROLES = ICI / "templates" / "theme-roles.json"
DERIVE = "couleurs et code sont dérivés de palette par scripts/themes_palettes.py (rôles : templates/theme-roles.json) : ne pas les éditer à la main"
# La teinte va vers la visée d'une part de l'écart, au plus tant de degrés : la couleur reste celle du thème.
DECALAGE = (0.35, 20.0)
ACCOMPAGNE = {"comment", "punctuation", "cadre"}


class Refus(Exception):
    pass


# --- OKLCH (Björn Ottosson, « A perceptual color space for image processing », 2020) ---

def _lineaire(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def _gamma(c):
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def hex_vers_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))


def rgb_vers_oklch(rgb):
    r, g, b = (_lineaire(c) for c in rgb)
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l_, m_, s_ = (math.copysign(abs(x) ** (1 / 3), x) for x in (l, m, s))
    L = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_
    a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_
    bb = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_
    return (L, math.hypot(a, bb), math.degrees(math.atan2(bb, a)) % 360)


def oklch_vers_rgb_lineaire(L, C, h):
    a, bb = C * math.cos(math.radians(h)), C * math.sin(math.radians(h))
    l_ = L + 0.3963377774 * a + 0.2158037573 * bb
    m_ = L - 0.1055613458 * a - 0.0638541728 * bb
    s_ = L - 0.0894841775 * a - 1.2914855480 * bb
    l, m, s = l_ ** 3, m_ ** 3, s_ ** 3
    return (4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
            -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
            -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s)


def dans_la_gamme(L, C, h, eps=1e-6):
    return all(-eps <= c <= 1 + eps for c in oklch_vers_rgb_lineaire(L, C, h))


def dans_srgb(L, C, h):
    """La couleur, ramenée dans sRGB en ôtant de la chroma (teinte et clarté gardées)."""
    L = min(max(L, 0.0), 1.0)
    if dans_la_gamme(L, C, h):
        return (L, C, h)
    bas, haut = 0.0, C
    for _ in range(40):
        milieu = (bas + haut) / 2
        if dans_la_gamme(L, milieu, h):
            bas = milieu
        else:
            haut = milieu
    return (L, bas, h)


def oklch_vers_hex(L, C, h):
    L, C, h = dans_srgb(L, C, h)
    canaux = (min(max(_gamma(min(max(c, 0.0), 1.0)), 0.0), 1.0) for c in oklch_vers_rgb_lineaire(L, C, h))
    return "#" + "".join(f"{round(c * 255):02x}" for c in canaux)


def canonique(oklch):
    L, C, h = oklch
    return [round(L, 5), round(C, 5), round(h % 360, 3)]


# --- WCAG 2 ---

def luminance(hexa):
    r, g, b = (_lineaire(c) for c in hex_vers_rgb(hexa))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contraste(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def au_seuil(L, C, h, fond, seuil, quoi):
    """La couleur, sa clarté éloignée du fond d'un centième à la fois jusqu'au seuil ; un refus sinon."""
    sens = 1 if luminance(fond) < 0.18 else -1
    while 0.0 <= L <= 1.0:
        hexa = oklch_vers_hex(L, C, h)
        if contraste(hexa, fond) >= seuil:
            return hexa
        L = round(L + sens * 0.01, 4)
    raise Refus(f"{quoi} : aucune clarté n'atteint {seuil} contre le fond {fond}")


# --- la dérivation ---

def ecart_de_teinte(a, b):
    return (b - a + 180) % 360 - 180


def couleur_de(palette, ref, clartes):
    """Une référence de palette : « accent », ou « accent.3 », sa nuance (clarté de l'échelle)."""
    nom_de_role, _, nuance = ref.partition(".")
    c = next((c for c in palette.values() if c["role"] == nom_de_role), None)
    if c is None:
        raise Refus(f"aucune couleur de rôle « {nom_de_role} » dans la palette")
    L, C, h = c["oklch"]
    if nuance:
        L = clartes[int(nuance) - 1]
    return (L, C, h)


def plus_proche(palette, visee):
    chromatiques = [c["oklch"] for c in palette.values() if c["role"] in ("accent", "halo", "halo2", "teinte") and c["oklch"][1] >= 0.04]
    if not chromatiques:
        raise Refus("aucune couleur chromatique dans la palette")
    return min(chromatiques, key=lambda o: abs(ecart_de_teinte(o[2], visee)))


def deriver(theme, roles_communs):
    mode = theme["mode"]
    commun = roles_communs[mode]
    clartes = roles_communs["nuances"]
    roles = {**commun, **theme.get("roles", {})}
    code_roles = {**commun["code"], **theme.get("roles", {}).get("code", {})}
    palette = theme["palette"]
    fond = oklch_vers_hex(*couleur_de(palette, roles["fond"], clartes))
    couleurs = {"fond": fond}
    couleurs["texte"] = au_seuil(*couleur_de(palette, roles["texte"], clartes), fond, 4.5, "texte")
    couleurs["halo"] = [oklch_vers_hex(*couleur_de(palette, r, clartes)) for r in roles["halo"]]
    couleurs["sourdine"] = au_seuil(*couleur_de(palette, roles["sourdine"], clartes), fond, 3.0, "sourdine")
    code = {}
    for sorte, regle in code_roles.items():
        if "depuis" in regle:
            L, C, h = couleur_de(palette, regle["depuis"], clartes)
        else:
            L, C, h = plus_proche(palette, regle["teinte"])
            part, borne = DECALAGE
            d = ecart_de_teinte(h, regle["teinte"])
            h = (h + max(-borne, min(borne, d * part))) % 360
        if "clarte" in regle:
            L = regle["clarte"]
        C = C * regle.get("chroma", 1.0)
        seuil = 3.0 if sorte in ACCOMPAGNE else 4.5
        code[sorte] = au_seuil(L, C, h, fond, seuil, f"code.{sorte}")
    return couleurs, code


def ecrit(theme, roles_communs):
    """Le thème tel que le script l'écrit : sa palette canonique, ses champs dérivés, la mention."""
    t = json.loads(json.dumps(theme))
    for c in t["palette"].values():
        if "hex" in c:
            c["oklch"] = canonique(rgb_vers_oklch(hex_vers_rgb(c.pop("hex"))))
        else:
            c["oklch"] = canonique(c["oklch"])
    couleurs, code = deriver(t, roles_communs)
    t["derive"] = DERIVE
    for k, v in couleurs.items():
        t["couleurs"][k] = v
    t["code"] = {**t.get("code", {}), **code}
    return t


def texte_json(t):
    return json.dumps(t, ensure_ascii=False, indent=2) + "\n"


def principal(verifier):
    roles = json.loads(ROLES.read_text())
    ecarts = []
    for f in sorted(THEMES.glob("*.json")):
        avant = f.read_text()
        apres = texte_json(ecrit(json.loads(avant), roles))
        if avant != apres:
            if verifier:
                ecarts.append(f.name)
            else:
                f.write_text(apres)
                print(f"réécrit : {f.name}")
    if ecarts:
        print("ROUGE : ces thèmes diffèrent de ce que dérive leur palette (lancer scripts/themes_palettes.py) :", ", ".join(ecarts))
        return 1
    print("VERT : chaque thème est celui que dérive sa palette" if verifier else "fait")
    return 0


def essais():
    rates = []

    def doit(cond, quoi):
        if not cond:
            rates.append(quoi)

    for h in ["#ff3333", "#050505", "#8fd16a", "#00f0ff", "#ffffff", "#000000", "#6a1fb0"]:
        doit(oklch_vers_hex(*canonique(rgb_vers_oklch(hex_vers_rgb(h)))) == h, f"aller-retour {h}")
    L, C, h = dans_srgb(0.7, 0.5, 140.0)
    doit((round(L, 6), round(h, 6)) == (0.7, 140.0) and C < 0.5 and dans_la_gamme(L, C, h), "la gamme : la chroma baisse, teinte et clarté gardées")
    doit(oklch_vers_hex(0.98, 0.3, 25.0) == oklch_vers_hex(*dans_srgb(0.98, 0.3, 25.0)), "une nuance hors gamme se ramène, jamais par canal")
    doit(abs(contraste("#ffffff", "#000000") - 21.0) < 1e-9, "contraste WCAG 2")
    sombre = au_seuil(0.3, 0.1, 30.0, "#050505", 4.5, "x")
    doit(contraste(sombre, "#050505") >= 4.5 and luminance(sombre) > luminance(oklch_vers_hex(0.3, 0.1, 30.0)), "sur fond sombre, la clarté monte jusqu'au seuil")
    clair = au_seuil(0.9, 0.1, 30.0, "#fafafa", 4.5, "x")
    doit(contraste(clair, "#fafafa") >= 4.5 and luminance(clair) < luminance("#fafafa"), "sur fond clair, la clarté descend")
    try:
        au_seuil(0.5, 0.0, 0.0, "#777777", 21.0, "impossible")
        rates.append("un seuil inatteignable doit refuser")
    except Refus:
        pass
    doit(abs(ecart_de_teinte(350, 10) - 20) < 1e-9 and abs(ecart_de_teinte(10, 350) + 20) < 1e-9, "l'écart de teinte passe par 0°")
    palette = {"a": {"oklch": [0.6, 0.2, 30.0], "role": "accent"}, "b": {"oklch": [0.7, 0.15, 140.0], "role": "teinte"}, "n": {"oklch": [0.5, 0.0, 0.0], "role": "neutre"}}
    doit(plus_proche(palette, 120.0) == [0.7, 0.15, 140.0], "la couleur la plus proche en teinte")
    doit(couleur_de(palette, "accent.3", [0.1 * i for i in range(1, 10)])[0] == 0.30000000000000004 or abs(couleur_de(palette, "accent.3", [0.1 * i for i in range(1, 10)])[0] - 0.3) < 1e-9, "une nuance prend la clarté de l'échelle")
    roles = json.loads(ROLES.read_text())
    for mode in ("sombre", "clair"):
        doit(mode in roles and "code" in roles[mode], f"les rôles du mode {mode}")
    clair_theme = {"mode": "clair", "couleurs": {}, "palette": {"f": {"oklch": [0.98, 0.005, 90.0], "role": "fond"}, "n": {"oklch": [0.55, 0.01, 90.0], "role": "neutre"}, "a": {"oklch": [0.5, 0.2, 260.0], "role": "accent"}, "h": {"oklch": [0.6, 0.2, 30.0], "role": "halo"}, "h2": {"oklch": [0.8, 0.1, 30.0], "role": "halo2"}, "t": {"oklch": [0.6, 0.15, 140.0], "role": "teinte"}}}
    try:
        couleurs, code = deriver(clair_theme, roles)
        doit(all(contraste(c, couleurs["fond"]) >= (3.0 if s in ACCOMPAGNE else 4.5) for s, c in code.items()), "un thème clair tient ses seuils")
    except Refus as e:
        rates.append(f"un thème clair se dérive : {e}")
    if rates:
        print("ROUGE :", "; ".join(rates))
        return 1
    print("VERT : les essais du script")
    return 0


if __name__ == "__main__":
    arg = sys.argv[1] if len(sys.argv) > 1 else ""
    try:
        sys.exit(essais() if arg == "--essais" else principal(arg == "--check"))
    except Refus as e:
        print(f"ROUGE : {e}")
        sys.exit(1)
