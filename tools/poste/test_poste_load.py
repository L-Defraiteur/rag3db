#!/usr/bin/env python3
"""Witness of tools/poste/poste_load.py: a fake /proc, /sys and poste --qui, two runs, then the rotation.

Usage: python3 tools/poste/test_poste_load.py [copy of poste_load.py] — exit 0 when every case is green.
"""
import os
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SCRIPT = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "poste_load.py"
failures, count = [], 0


def case(name, ok, detail=""):
    global count
    count += 1
    print(("vert   " if ok else "ROUGE  ") + name + ("" if ok else f" — {detail}"))
    if not ok:
        failures.append(name)


def write(p, text):
    p.parent.mkdir(parents=True, exist_ok=True); p.write_text(text, encoding="utf-8")


def run(t, now, stat):
    write(t / "proc" / "stat", stat)
    env = dict(os.environ, POSTE_LOAD_DIR=str(t / "out"), POSTE_LOAD_PROC=str(t / "proc"), POSTE_LOAD_SYS=str(t / "sys"),
               POSTE_LOAD_POSTE=str(t / "poste"), POSTE_LOAD_NOW=now, POSTE_LOCK_DIR=str(t / "locks"))
    return subprocess.run([sys.executable, "-I", str(SCRIPT)], env=env, capture_output=True, text=True)


def rows(t):
    lines = (t / "out" / "poste-load.tsv").read_text(encoding="utf-8").splitlines()
    head = lines[0].split("\t")
    return head, [dict(zip(head, l.split("\t"))) for l in lines[1:]]


with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    write(t / "proc" / "loadavg", "12.50 11.00 10.00 3/900 1234\n")
    write(t / "proc" / "meminfo", "MemTotal: 100663296 kB\nMemAvailable: 50331648 kB\nSwapTotal: 8388608 kB\nSwapFree: 6291456 kB\n")
    write(t / "proc" / "pressure" / "memory", "some avg10=1.50 avg60=0.36 avg300=0.60 total=1\nfull avg10=0.25 avg60=0.33 avg300=0.58 total=1\n")
    write(t / "proc" / "pressure" / "cpu", "some avg10=7.00 avg60=0.17 avg300=1.48 total=1\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n")
    write(t / "sys" / "devices" / "system" / "cpu" / "online", "0-23\n")
    for card, busy, used in (("card0", "100", 2 * 1024 ** 3), ("card2", "40", 16 * 1024 ** 3)):
        dev = t / "sys" / "class" / "drm" / card / "device"
        write(dev / "gpu_busy_percent", busy + "\n"); write(dev / "mem_info_vram_used", f"{used}\n"); write(dev / "mem_info_vram_total", f"{32 * 1024 ** 3}\n")
    write(t / "sys" / "class" / "drm" / "card1" / "device" / "vendor", "0x8086\n")  # an Intel card: no gpu_busy_percent
    write(t / "locks" / "coeurs-libres", "2\n")
    write(t / "poste", "#!/bin/sh\ncat <<'Q'\ntient lourd depuis 11:33:32 (pid 1) — a\ntient cpu depuis 11:34:00 (pid 2) — b\n"
                       "tient gpu+cpu 12G depuis 11:35:00 (pid 3) — c\nattend cpu depuis 11:34:20 (pid 4) — d\n"
                       "tient gpu 6G depuis 11:36:00 (pid 5) — e\nattend gpu 20G depuis 11:37:00 (pid 6) — f\n"
                       "lourds : 3/4 places tenues, en file dans l'ordre : pid 4\nQ\n")
    (t / "poste").chmod(0o755)

    if not SCRIPT.is_file():
        case("le script existe", False, str(SCRIPT))
    else:
        # user+nice+system busy, idle+iowait idle: first run has no interval, second has 300 busy of 1000.
        r1 = run(t, "2026-11-21T12:00:00", "cpu  1000 0 0 9000 0 0 0 0 0 0\n")
        r2 = run(t, "2026-11-21T12:02:00", "cpu  1200 0 100 9600 100 0 0 0 0 0\n")
        case("deux passages sans erreur", r1.returncode == 0 and r2.returncode == 0, r1.stderr + r2.stderr)
        head, rs = rows(t)
        case("un en-tête, puis une ligne par passage", head[0] == "time" and len(rs) == 2, str(rs))
        a, b = rs
        case("le premier passage n'a pas d'intervalle : CPU « - »", a["cpu_busy_pct"] == "-", a["cpu_busy_pct"])
        case("CPU occupé sur l'intervalle, depuis /proc/stat (300/1000 = 30 %)", b["cpu_busy_pct"] == "30.0", b["cpu_busy_pct"])
        case("loadavg 1 min et cœurs", b["load1"] == "12.50" and b["cores"] == "24", f'{b["load1"]} {b["cores"]}')
        case("RAM utilisée et disponible, swap", (b["ram_used_gib"], b["ram_available_gib"], b["swap_used_gib"], b["swap_total_gib"]) == ("48.0", "48.0", "2.0", "8.0"), str(b))
        case("pression mémoire et CPU", (b["psi_mem_some10"], b["psi_mem_full10"], b["psi_cpu_some10"]) == ("1.50", "0.25", "7.00"), str(b))
        case("places cpu : 3 tenues (lourd, cpu, gpu+cpu), 4 réglées, 1 en attente", (b["heavy_held"], b["heavy_places"], b["heavy_waiting"]) == ("3", "4", "1"), str(b))
        case("gpu : 2 tenus (gpu+cpu 12G, gpu 6G), 1 en attente, 18 Gio déclarés", (b["gpu_held"], b["gpu_waiting"], b["gpu_declared_gib"]) == ("2", "1", "18"), str(b))
        case("enveloppe d'un lourd : (24 - 2) / 4 = 5,5 cœurs", b["heavy_cores"] == "5.5", b["heavy_cores"])
        case("cartes : card0 et card2, pas la carte Intel", b["gpus"] == "card0:100%:2.0/32.0,card2:40%:16.0/32.0", b["gpus"])
        # A poste that does not answer: « - », never a crash.
        write(t / "poste", "#!/bin/sh\nexit 1\n")
        r3 = run(t, "2026-11-21T12:04:00", "cpu  1300 0 100 9700 100 0 0 0 0 0\n")
        _, rs = rows(t)
        case("poste muet : places dites « - », la ligne écrite", r3.returncode == 0 and rs[-1]["heavy_places"] == "-" and rs[-1]["heavy_held"] == "0", str(rs[-1]))
        # Rotation: fourteen days kept.
        r4 = run(t, "2026-12-05T12:03:00", "cpu  1400 0 100 9800 100 0 0 0 0 0\n")
        _, rs = rows(t)
        case("rotation : les lignes de plus de 14 jours sont retirées", r4.returncode == 0 and [x["time"] for x in rs] == ["2026-11-21T12:04:00", "2026-12-05T12:03:00"], str([x["time"] for x in rs]))
        # A file under older columns is moved aside, and a new one starts with the current header.
        p = t / "out" / "poste-load.tsv"
        p.write_text("time\tcpu_busy_pct\n2026-11-21T12:00:00\t1.0\n", encoding="utf-8")
        r5 = run(t, "2026-12-05T12:05:00", "cpu  1310 0 100 9710 100 0 0 0 0 0\n")
        head, rs = rows(t)
        aside = list((t / "out").glob("poste-load.tsv.before-*"))
        case("colonnes changées : l'ancien fichier mis de côté, un nouveau commence", r5.returncode == 0 and head[-1] == "gpu_declared_gib" and len(rs) == 1 and len(aside) == 1, f"{head} {len(rs)} {aside}")

print(f"{count - len(failures)}/{count} verts")
sys.exit(1 if failures else 0)
