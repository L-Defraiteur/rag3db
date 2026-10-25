#!/usr/bin/env python3
"""**Le magasin de libs par identifiant de bâti** (page
docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md).

Un dossier par identifiant, immuable une fois publié, sous
`${RAG3DB_LIBS_DIR:-${XDG_CACHE_HOME:-~/.cache}/rag3db-libs}` :

    <id>/librag3db.so, rag3db.hpp, include/common/build_id.h,
         libvector.rag3db_extension, libgeo.rag3db_extension (si bâties),
         BUILD_ID, MANIFESTE (json : identifiant, date, source, md5)
    courant -> <id>          (les passes le résolvent une fois au départ)

Sous-commandes :
    publier <dossier de bâti> [--racine <arbre>] [--sans-courant]
    courant [--poser <id>]   rend le dossier résolu, ou change le lien
    verifier <dossier|id>    la lib contient-elle les planchers (planchers-de-la-lib) ?
    liste                    les identifiants, et ce qui retient chacun
    menage [--garder N] [--simuler]

Un binaire installé suit un identifiant fixe (son RUNPATH), jamais `courant`.
Le ménage épargne les N plus récents, `courant`, un dossier `ÉPINGLÉ`, un
dossier mappé par un processus vivant (/proc/*/maps), et un dossier visé par
le RUNPATH d'un binaire de ~/.local/bin (ou de RAG3DB_LIBS_BINS) ou du binaire
d'une unité utilisateur active ou activée."""
import argparse
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import time
from pathlib import Path

LIB_SECTION = ".rag3db_build_id"
EXTENSIONS = ["vector", "geo"]


def magasin() -> Path:
    if os.environ.get("RAG3DB_LIBS_DIR"):
        return Path(os.environ["RAG3DB_LIBS_DIR"])
    cache = os.environ.get("XDG_CACHE_HOME") or str(Path.home() / ".cache")
    return Path(cache) / "rag3db-libs"


def refus(message: str) -> None:
    print(f"rag3db_libs : {message}", file=sys.stderr)
    sys.exit(2)


# ── ELF : une section, et le RUNPATH, sans rien charger ────────────────────

def _sections(path: Path):
    with open(path, "rb") as f:
        h = f.read(64)
        if h[:4] != b"\x7fELF" or h[4] != 2 or h[5] != 1:
            return None, []
        shoff, = struct.unpack_from("<Q", h, 0x28)
        shentsize, shnum, shstrndx = struct.unpack_from("<HHH", h, 0x3A)
        f.seek(shoff)
        table = f.read(shentsize * shnum)
        secs = []
        for i in range(shnum):
            name, typ, _flags, _addr, off, size, link = struct.unpack_from("<IIQQQQI", table, i * shentsize)
            secs.append((name, typ, off, size, link))
        _, _, stroff, strsize, _ = secs[shstrndx]
        f.seek(stroff)
        names = f.read(strsize)
    def nom(n):
        return names[n:names.index(b"\0", n)].decode(errors="replace")
    return nom, secs


def section(path: Path, wanted: str) -> str | None:
    nom, secs = _sections(path)
    if nom is None:
        return None
    for n, _typ, off, size, _link in secs:
        if nom(n) == wanted:
            with open(path, "rb") as f:
                f.seek(off)
                return f.read(size).split(b"\0")[0].decode()
    return None


def runpaths(path: Path) -> list[str]:
    """Le RUNPATH (ou RPATH) d'un ELF, en dossiers ; vide sinon."""
    try:
        nom, secs = _sections(path)
    except OSError:
        return []
    if nom is None:
        return []
    for _n, typ, off, size, link in secs:
        if typ != 6:  # SHT_DYNAMIC
            continue
        with open(path, "rb") as f:
            f.seek(off)
            dyn = f.read(size)
            _, _, stroff, strsize, _ = secs[link]
            f.seek(stroff)
            strtab = f.read(strsize)
        out = []
        for i in range(0, len(dyn) - 15, 16):
            tag, val = struct.unpack_from("<qQ", dyn, i)
            if tag in (15, 29):  # DT_RPATH, DT_RUNPATH
                out += strtab[val:strtab.index(b"\0", val)].decode().split(":")
        return out
    return []


def md5(path: Path) -> str:
    h = hashlib.md5()
    with open(path, "rb") as f:
        for bloc in iter(lambda: f.read(1 << 20), b""):
            h.update(bloc)
    return h.hexdigest()


# ── publier ─────────────────────────────────────────────────────────────────

def publier(bati: Path, racine: Path, poser_courant: bool) -> None:
    src = bati / "src"
    lib = src / "librag3db.so"
    entete = src / "include/common/build_id.h"
    for f in [lib, src / "rag3db.hpp", entete]:
        if not f.exists():
            refus(f"{f} manque : ce n'est pas un dossier de bâti du moteur partagé")
    ident = section(lib, LIB_SECTION)
    if not ident:
        refus(f"{lib} ne porte pas son identifiant (section {LIB_SECTION}) : une lib d'avant f5fdc7e9a")
    m = re.search(r'#define\s+RAG3DB_BUILD_ID\s+"([^"]+)"', entete.read_text())
    if not m or m.group(1) != ident:
        refus(f"l'en-tête {entete} dit {m.group(1) if m else 'rien'}, la lib dit {ident} : ce n'est pas le même bâti")
    fichiers = {"librag3db.so": lib, "rag3db.hpp": src / "rag3db.hpp", "include/common/build_id.h": entete}
    for ext in EXTENSIONS:
        e = racine / f"extension/{ext}/build/lib{ext}.rag3db_extension"
        if not e.exists():
            continue
        if ident.encode() not in e.read_bytes():
            refus(f"l'extension {e} (lib{ext}) n'est pas du bâti {ident} : la paire lib et extension n'irait pas ensemble")
        fichiers[e.name] = e

    m_dir = magasin()
    m_dir.mkdir(parents=True, exist_ok=True)
    cible = m_dir / ident
    sommes = {nom: md5(p) for nom, p in fichiers.items()}
    if cible.exists():
        deja = json.loads((cible / "MANIFESTE").read_text())
        if deja["md5"].get("librag3db.so") != sommes["librag3db.so"]:
            refus(f"{ident} est déjà publié avec une autre lib (md5 {deja['md5'].get('librag3db.so')} contre "
                  f"{sommes['librag3db.so']}) : un identifiant ne dit pas tout ce qui change l'ABI ; rien n'est écrasé")
        print(f"déjà publié : {cible}")
    else:
        tmp = m_dir / f".tmp-{os.getpid()}"
        shutil.rmtree(tmp, ignore_errors=True)
        for nom, p in fichiers.items():
            (tmp / nom).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(p, tmp / nom)
        (tmp / "BUILD_ID").write_text(ident + "\n")
        (tmp / "MANIFESTE").write_text(json.dumps({
            "identifiant": ident,
            "publie_le": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "publie_ns": time.time_ns(),
            "source": str(bati.resolve()),
            "md5": sommes,
        }, indent=2, ensure_ascii=False) + "\n")
        os.rename(tmp, cible)
        print(f"publié : {cible}")
    if poser_courant:
        poser(ident)


# ── les planchers : les commits du moteur qu'une lib doit contenir ──────────

PLANCHERS = Path(__file__).resolve().parent / "planchers-de-la-lib"


def planchers() -> list[tuple[str, str]]:
    """(commit, raison) de `planchers-de-la-lib` ; aucun fichier : aucun plancher."""
    if not PLANCHERS.exists():
        return []
    out = []
    for ligne in PLANCHERS.read_text().splitlines():
        ligne = ligne.strip()
        if not ligne or ligne.startswith("#"):
            continue
        commit, _, raison = ligne.partition(" ")
        out.append((commit, raison.strip()))
    return out


def commit_du_bati(ident: str) -> str | None:
    m = re.match(r"rag3db-build-([0-9a-f]{40})-", ident)
    return m.group(1) if m else None


def manques(ident: str) -> tuple[list[tuple[str, str]], list[str]]:
    """Les planchers que la lib `ident` ne contient pas, et ce qui n'a pas pu
    se dire (un commit que le dépôt de ce script ne connaît pas : dit, pas
    refusé — on ne refuse que ce que git prouve absent)."""
    commit = commit_du_bati(ident)
    if not commit:
        return [], [f"{ident} : pas de commit dans l'identifiant, planchers non vérifiés"]
    depot = Path(__file__).resolve().parent
    absents, inconnus = [], []
    for plancher, raison in planchers():
        r = subprocess.run(["git", "-C", str(depot), "merge-base", "--is-ancestor", plancher, commit],
                           capture_output=True, text=True)
        if r.returncode == 1:
            absents.append((plancher, raison))
        elif r.returncode != 0:
            inconnus.append(f"plancher {plancher[:9]} contre {commit[:9]} : git ne sait pas ({r.stderr.strip()})")
    return absents, inconnus


def verifier(cible: str) -> None:
    """Refuse (code 2) une lib qui ne contient pas un plancher, en le nommant."""
    d = Path(cible)
    ident = (d / "BUILD_ID").read_text().strip() if (d / "BUILD_ID").exists() else d.name
    absents, inconnus = manques(ident)
    for i in inconnus:
        print(f"⚠ {i}", file=sys.stderr)
    if absents:
        refus(f"la lib {ident} ne contient pas " + " ; ni ".join(f"{c[:9]} — {r}" for c, r in absents)
              + " : prenez courant, ou une lib bâtie après")


def poser(ident: str) -> None:
    m_dir = magasin()
    if not (m_dir / ident).is_dir():
        refus(f"{ident} n'est pas au magasin {m_dir}")
    # `courant` ne se pose jamais sur une lib sous un plancher : toutes les
    # passes la prendraient.
    verifier(str(m_dir / ident))
    tmp = m_dir / f".courant-{os.getpid()}"
    if tmp.is_symlink() or tmp.exists():
        tmp.unlink()
    os.symlink(ident, tmp)
    os.replace(tmp, m_dir / "courant")
    print(f"courant -> {ident}")


# ── ce qui retient un identifiant ───────────────────────────────────────────

def mappes(m_dir: Path) -> dict[str, str]:
    """Identifiant → un processus qui en mappe un fichier."""
    out = {}
    prefixe = str(m_dir.resolve()) + "/"
    for pid in filter(str.isdigit, os.listdir("/proc")):
        try:
            texte = open(f"/proc/{pid}/maps").read()
        except OSError:
            continue
        for ligne in texte.splitlines():
            champs = ligne.split(None, 5)
            if len(champs) == 6 and champs[5].startswith(prefixe):
                out.setdefault(champs[5][len(prefixe):].split("/")[0], f"pid {pid}")
    return out


def binaires_installes() -> list[Path]:
    dirs = os.environ.get("RAG3DB_LIBS_BINS") or str(Path.home() / ".local/bin")
    out = [p for d in dirs.split(":") if Path(d).is_dir() for p in Path(d).iterdir() if p.is_file()]
    if not os.environ.get("RAG3DB_LIBS_SANS_UNITES"):
        out += binaires_des_unites()
    return out


def binaires_des_unites() -> list[Path]:
    """Le binaire d'ExecStart de chaque unité utilisateur active ou activée."""
    def systemctl(*args):
        try:
            return subprocess.run(["systemctl", "--user", *args], capture_output=True, text=True, timeout=10).stdout
        except (OSError, subprocess.TimeoutExpired):
            return ""
    unites = set()
    for l in systemctl("list-units", "--type=service", "--state=active", "--no-legend", "--plain").splitlines():
        if l.split():
            unites.add(l.split()[0])
    for l in systemctl("list-unit-files", "--type=service", "--state=enabled", "--no-legend").splitlines():
        if l.split():
            unites.add(l.split()[0])
    out = []
    for u in unites:
        for m in re.finditer(r"path=([^ ;]+)", systemctl("show", "-p", "ExecStart", "--value", u)):
            out.append(Path(m.group(1)))
    return out


def references(m_dir: Path) -> dict[str, str]:
    """Identifiant → un binaire installé dont le RUNPATH le vise."""
    out = {}
    base = str(m_dir.resolve()) + "/"
    brut = str(m_dir) + "/"
    for b in binaires_installes():
        for rp in runpaths(b):
            for prefixe in (base, brut):
                if rp.startswith(prefixe):
                    out.setdefault(rp[len(prefixe):].split("/")[0], str(b))
    return out


def identifiants(m_dir: Path) -> list[tuple[str, int]]:
    out = []
    for d in m_dir.iterdir() if m_dir.is_dir() else []:
        if d.is_dir() and not d.is_symlink() and not d.name.startswith(".") and (d / "MANIFESTE").exists():
            out.append((d.name, json.loads((d / "MANIFESTE").read_text()).get("publie_ns", 0)))
    return sorted(out, key=lambda x: x[1], reverse=True)


def raisons(m_dir: Path, garder: int) -> dict[str, list[str]]:
    ids = identifiants(m_dir)
    cour = os.path.basename(os.path.realpath(m_dir / "courant")) if (m_dir / "courant").exists() else None
    map_ = mappes(m_dir)
    ref = references(m_dir)
    out = {}
    for rang, (i, _) in enumerate(ids):
        r = []
        if rang < garder:
            r.append(f"parmi les {garder} plus récents")
        if i == cour:
            r.append("courant")
        if (m_dir / i / "ÉPINGLÉ").exists():
            r.append("épinglé")
        if i in map_:
            r.append(f"mappé ({map_[i]})")
        if i in ref:
            r.append(f"référencé ({ref[i]})")
        out[i] = r
    return out


def menage(garder: int, simuler: bool) -> None:
    m_dir = magasin()
    for i, r in raisons(m_dir, garder).items():
        if r:
            print(f"gardé  {i} : {', '.join(r)}")
        else:
            print(f"{'à effacer' if simuler else 'effacé'} {i}")
            if not simuler:
                shutil.rmtree(m_dir / i)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    pp = sub.add_parser("publier")
    pp.add_argument("bati", type=Path)
    pp.add_argument("--racine", type=Path, default=Path(__file__).resolve().parents[3])
    pp.add_argument("--sans-courant", action="store_true")
    pc = sub.add_parser("courant")
    pc.add_argument("--poser")
    sub.add_parser("liste")
    pv = sub.add_parser("verifier")
    pv.add_argument("cible")
    pm = sub.add_parser("menage")
    pm.add_argument("--garder", type=int, default=10)
    pm.add_argument("--simuler", action="store_true")
    a = p.parse_args()
    if a.cmd == "publier":
        publier(a.bati, a.racine, not a.sans_courant)
    elif a.cmd == "courant":
        if a.poser:
            poser(a.poser)
        else:
            c = magasin() / "courant"
            if not c.exists():
                refus(f"pas de lien courant dans {magasin()}")
            print(os.path.realpath(c))
    elif a.cmd == "liste":
        for i, r in raisons(magasin(), 10).items():
            absents, _ = manques(i)
            sous = f" — SOUS LE PLANCHER {', '.join(c[:9] for c, _ in absents)}" if absents else ""
            print(f"{i} : {', '.join(r) or 'rien ne le retient'}{sous}")
    elif a.cmd == "verifier":
        verifier(a.cible)
    elif a.cmd == "menage":
        menage(a.garder, a.simuler)


if __name__ == "__main__":
    main()
