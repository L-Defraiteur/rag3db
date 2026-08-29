#!/usr/bin/env python3
"""**Ce que la machine paie pendant qu'on travaille.**

Un échantillon par intervalle, écrit dans un fichier — jamais en bout de
pipe (voir la règle « écrire d'abord, filtrer ensuite »). On peut donc faire
un `tail -f` dessus pendant une compilation ou une passe E2E, et surtout on
garde la trace après coup, quand on cherche pourquoi le poste a figé.

Ce qu'on mesure, et pourquoi :

- **`io_full`** en premier, et de loin. C'est la *pression* d'entrées-sorties
  que Linux publie dans `/proc/pressure/io` : le pourcentage de temps pendant
  lequel **toutes** les tâches non oisives sont bloquées à attendre le disque.
  Pas « le disque travaille » — « plus personne n'avance ». C'est la seule
  mesure qui répond directement à « mon PC rame » ; charge, CPU et RAM peuvent
  toutes être basses pendant que celle-ci est à 60. Mesuré le 28 août 2026 :
  `io_full = 60 %` pendant une passe, avec le CPU à 11 % et la mémoire à 0 de
  pression.
- **`sales`**, les pages en attente d'écriture. CachyOS plafonne
  `vm.dirty_bytes` à **256 Mo** (`/usr/lib/sysctl.d/70-cachyos-settings.conf`)
  — un réglage de latence pour un bureau, qui suppose de petites écritures.
  Une passe en écrit 48 Go : au-delà du plafond, chaque `write()` bloque
  jusqu'à ce que le noyau ait vidé. Voir cette colonne osciller en dents de
  scie contre 256 Mo, c'est voir la cause.
- **`swap`** ensuite. C'est lui qui fait « galérer » un poste avec 96 Go
  de RAM : quand la compilation gonfle, des pages du bureau partent au
  disque et ne reviennent qu'à coups de défauts de page. Mesuré le 27 août :
  43,9 Go de swap occupés pendant que 35 Go de RAM étaient libres.
- **`load`** rapporté au nombre de cœurs — un `load` de 50 sur 24 cœurs veut
  dire qu'on attend deux fois plus qu'on ne calcule.
- **le GPU par carte**, occupation et VRAM. `card2` porte les écrans de
  travail, `card0` fait tourner Qwen (doc 13 §4) : voir laquelle chauffe dit
  tout de suite si c'est l'embedder ou l'affichage qui souffre.
- **les trois plus gros par mémoire résidente**, pour nommer le coupable.

Usage :
    ./charge.py --sortie target/charge.tsv --intervalle 5
    ./charge.py --une-fois          # une ligne, puis on sort
"""

import argparse
import os
import sys
import time

CARTES = sorted(
    d for d in os.listdir("/sys/class/drm")
    if d.startswith("card") and "-" not in d
    and os.path.exists(f"/sys/class/drm/{d}/device/gpu_busy_percent")
)


def lire(chemin, defaut=""):
    try:
        with open(chemin) as f:
            return f.read().strip()
    except OSError:
        return defaut


def meminfo():
    champs = {}
    for ligne in lire("/proc/meminfo").splitlines():
        cle, _, reste = ligne.partition(":")
        champs[cle] = int(reste.split()[0]) // 1024  # Mio
    return champs


def pression(quoi):
    """`some`/`full` à 10 s pour `cpu`, `memory` ou `io`.

    `full` est celle qui compte : `some` dit qu'une tâche attend, `full` dit
    qu'aucune n'avance.
    """
    texte = lire(f"/proc/pressure/{quoi}")
    out = {}
    for ligne in texte.splitlines():
        parts = ligne.split()
        if not parts:
            continue
        for p in parts[1:]:
            cle, _, val = p.partition("=")
            if cle == "avg10":
                out[parts[0]] = val
    return out


def cpu_jiffies():
    parts = lire("/proc/stat").splitlines()[0].split()[1:]
    valeurs = [int(v) for v in parts]
    inactif = valeurs[3] + valeurs[4]
    return sum(valeurs), inactif


def gros_processus(n=3):
    """Les `n` plus gros par mémoire résidente, sans `ps` ni pipe."""
    sortie = []
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        statm = lire(f"/proc/{pid}/statm")
        if not statm:
            continue
        try:
            rss_mo = int(statm.split()[1]) * os.sysconf("SC_PAGE_SIZE") // (1024 * 1024)
        except (ValueError, IndexError):
            continue
        if rss_mo < 200:
            continue
        nom = lire(f"/proc/{pid}/comm", "?")
        sortie.append((rss_mo, nom))
    sortie.sort(reverse=True)
    return sortie[:n]


ENTETE = [
    "heure", "io_full", "io_some", "mem_full", "cpu_some", "sales_mo",
    "load1", "load/coeur", "cpu%", "ram_mo", "ram%", "swap_mo", "swap%",
]
for _c in CARTES:
    ENTETE += [f"{_c}_gpu%", f"{_c}_vram_mo"]
ENTETE += ["les_plus_gros"]


def echantillon(precedent):
    m = meminfo()
    total, inactif = cpu_jiffies()
    cpu = ""
    if precedent:
        d_total = total - precedent[0]
        d_inactif = inactif - precedent[1]
        if d_total > 0:
            cpu = f"{100.0 * (d_total - d_inactif) / d_total:.0f}"

    load1 = float(lire("/proc/loadavg", "0").split()[0])
    coeurs = os.cpu_count() or 1
    ram_util = m["MemTotal"] - m["MemAvailable"]
    swap_util = m["SwapTotal"] - m["SwapFree"]

    io = pression("io")
    mem = pression("memory")
    cpu_p = pression("cpu")
    sales = m.get("Dirty", 0) + m.get("Writeback", 0)

    ligne = [
        time.strftime("%H:%M:%S"),
        io.get("full", "?"),
        io.get("some", "?"),
        mem.get("full", "?"),
        cpu_p.get("some", "?"),
        str(sales),
        f"{load1:.1f}",
        f"{load1 / coeurs:.2f}",
        cpu,
        str(ram_util),
        f"{100 * ram_util // max(m['MemTotal'], 1)}",
        str(swap_util),
        f"{100 * swap_util // max(m['SwapTotal'], 1)}" if m["SwapTotal"] else "0",
    ]
    for carte in CARTES:
        d = f"/sys/class/drm/{carte}/device"
        ligne.append(lire(f"{d}/gpu_busy_percent", "?"))
        vram = lire(f"{d}/mem_info_vram_used", "")
        ligne.append(str(int(vram) // (1024 * 1024)) if vram.isdigit() else "?")
    ligne.append(" ".join(f"{nom}:{rss}Mo" for rss, nom in gros_processus()))
    return ligne, (total, inactif)


def resume(chemin):
    """Les pics d'une passe, en six lignes — ce qu'on regarde après coup.

    On ne relit pas cent échantillons : on veut savoir jusqu'où c'est monté,
    et **quand**. L'heure du pic de swap dit à quel moment de la passe le
    poste a commencé à ramer.
    """
    with open(chemin) as f:
        lignes = [l.rstrip("\n").split("\t") for l in f if l.strip()]
    if len(lignes) < 2:
        print("  (pas d'échantillon)")
        return
    entete, echantillons = lignes[0], [l for l in lignes[1:] if l[0] != "heure"]
    if not echantillons:
        print("  (pas d'échantillon)")
        return

    def pic(colonne, unite=""):
        try:
            i = entete.index(colonne)
        except ValueError:
            return None
        candidats = [(float(e[i]), e[0]) for e in echantillons if i < len(e) and e[i] not in ("", "?")]
        if not candidats:
            return None
        v, quand = max(candidats)
        return f"{v:g}{unite} (à {quand})"

    print(f"  {len(echantillons)} échantillons, de {echantillons[0][0]} à {echantillons[-1][0]}")
    for colonne, libelle, unite in [
        ("io_full", "TOUT bloqué sur le disque", " %"),
        ("io_some", "quelqu'un bloqué disque", " %"),
        ("sales_mo", "pages sales", " Mo"),
        ("mem_full", "tout bloqué mémoire", " %"),
        ("cpu_some", "quelqu'un en attente CPU", " %"),
        ("load/coeur", "charge par cœur", ""),
        ("cpu%", "CPU", " %"),
        ("ram%", "RAM", " %"),
        ("swap_mo", "swap", " Mo"),
    ]:
        p = pic(colonne, unite)
        if p:
            print(f"  pic {libelle:<16} {p}")
    for carte in CARTES:
        g, v = pic(f"{carte}_gpu%", " %"), pic(f"{carte}_vram_mo", " Mo")
        if g or v:
            print(f"  pic {carte:<16} {g or '?'} · VRAM {v or '?'}")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--sortie", default="-", help="fichier TSV (défaut : la sortie standard)")
    ap.add_argument("--intervalle", type=float, default=5.0)
    ap.add_argument("--une-fois", action="store_true")
    ap.add_argument("--resume", metavar="TSV", help="lire un journal et en donner les pics")
    args = ap.parse_args()

    if args.resume:
        resume(args.resume)
        return

    flux = sys.stdout if args.sortie == "-" else open(args.sortie, "a", buffering=1)
    if flux is not sys.stdout and flux.tell() == 0:
        print("\t".join(ENTETE), file=flux)
    elif flux is sys.stdout:
        print("\t".join(ENTETE), file=flux)

    precedent = None
    try:
        while True:
            ligne, precedent = echantillon(precedent)
            print("\t".join(ligne), file=flux)
            if args.une_fois:
                return
            time.sleep(args.intervalle)
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
