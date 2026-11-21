#!/usr/bin/env python3
"""The load of a poste, one line every two minutes (21 Nov. 2026, Lucie: « est-ce qu'on ne sous-utilise pas un peu
luciepc ? »), to judge whether the number of heavy places is too low.

Run by poste-load.timer (a systemd user unit, every 2 min). Appends one line to ~/.cache/rag3weaver-build/poste-load.tsv:

  time              local time, ISO
  cpu_busy_pct      CPU busy over the interval since the previous line (/proc/stat), all cores
  load1             loadavg over 1 min
  cores             online cores
  ram_used_gib      MemTotal - MemAvailable
  ram_available_gib MemAvailable
  swap_used_gib     SwapTotal - SwapFree (zram at luciepc)
  swap_total_gib
  psi_mem_some10    /proc/pressure/memory, some avg10
  psi_mem_full10    /proc/pressure/memory, full avg10
  psi_cpu_some10    /proc/pressure/cpu, some avg10
  heavy_held        cpu places held (poste --qui: « tient cpu », « tient gpu+cpu », the older « tient lourd »)
  heavy_places      cpu places set (« lourds : x/N places tenues »)
  heavy_waiting     cpu places waited for (« attend cpu », « attend gpu+cpu », « attend lourd »)
  heavy_cores       cores of one heavy envelope: (cores - coeurs-libres) / places
  gpus              each card exposing gpu_busy_percent: name:busy%:vram used/total GiB[:gtt used GiB]
  gpu_held          poste gpu jobs holding (« tient gpu… », 21 Nov. 2026: the classes)
  gpu_waiting       poste gpu jobs waiting
  gpu_declared_gib  Σ of the needs the holding gpu jobs declared
A file written under other columns is moved aside (poste-load.tsv.before-<time>) and a new one starts.

The file keeps 14 days: older lines are dropped. A missing source writes « - » in its column, never a crash.
Environment, for the witness: POSTE_LOAD_DIR (the folder of the file and its state), POSTE_LOAD_PROC (/proc),
POSTE_LOAD_SYS (/sys), POSTE_LOAD_POSTE (the poste command), POSTE_LOAD_NOW (the time, ISO).
"""
import datetime
import glob
import json
import os
import re
import subprocess

HOME = os.path.expanduser("~")
DIR = os.environ.get("POSTE_LOAD_DIR") or os.path.join(HOME, ".cache", "rag3weaver-build")
PROC = os.environ.get("POSTE_LOAD_PROC", "/proc")
SYS = os.environ.get("POSTE_LOAD_SYS", "/sys")
POSTE = os.environ.get("POSTE_LOAD_POSTE") or os.path.join(HOME, ".cache", "rag3weaver-build", "poste")
COLUMNS = ["time", "cpu_busy_pct", "load1", "cores", "ram_used_gib", "ram_available_gib", "swap_used_gib",
           "swap_total_gib", "psi_mem_some10", "psi_mem_full10", "psi_cpu_some10", "heavy_held", "heavy_places",
           "heavy_waiting", "heavy_cores", "gpus", "gpu_held", "gpu_waiting", "gpu_declared_gib"]
KEEP_DAYS = 14
GIB = 1024 ** 3


def read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError:
        return None


def cpu_times():
    """(busy, total) jiffies of the « cpu » line of /proc/stat."""
    text = read(os.path.join(PROC, "stat")) or ""
    for line in text.splitlines():
        if line.startswith("cpu "):
            v = [int(x) for x in line.split()[1:]]
            idle = v[3] + (v[4] if len(v) > 4 else 0)  # idle + iowait
            total = sum(v[:8])  # guest time is already in user
            return total - idle, total
    return None


def cpu_busy(state_path):
    now = cpu_times()
    if now is None:
        return "-"
    prev = None
    try:
        with open(state_path, encoding="utf-8") as f:
            prev = json.load(f).get("cpu")
    except (OSError, ValueError):
        pass
    with open(state_path + ".new", "w", encoding="utf-8") as f:
        json.dump({"cpu": list(now)}, f)
    os.replace(state_path + ".new", state_path)
    if not prev or now[1] <= prev[1]:
        return "-"
    return f"{100.0 * (now[0] - prev[0]) / (now[1] - prev[1]):.1f}"


def meminfo():
    m = {}
    for line in (read(os.path.join(PROC, "meminfo")) or "").splitlines():
        k, _, v = line.partition(":")
        if v.strip().endswith("kB"):
            m[k] = int(v.split()[0]) * 1024
    return m


def gib(x):
    return "-" if x is None else f"{x / GIB:.1f}"


def pressure(kind, line_kind):
    text = read(os.path.join(PROC, "pressure", kind)) or ""
    for line in text.splitlines():
        if line.startswith(line_kind + " "):
            m = re.search(r"avg10=([0-9.]+)", line)
            if m:
                return m.group(1)
    return "-"


def cores():
    text = read(os.path.join(SYS, "devices", "system", "cpu", "online"))
    if text:
        n = 0
        for part in text.strip().split(","):
            a, _, b = part.partition("-")
            n += (int(b) - int(a) + 1) if b else 1
        return n
    return os.cpu_count() or 0


CPU_PLACE = ("lourd", "cpu", "gpu+cpu")


def poste_qui():
    try:
        return subprocess.run([POSTE, "--qui"], capture_output=True, text=True, timeout=20).stdout
    except (OSError, subprocess.SubprocessError):
        return None


def heavy(n_cores, out):
    """(held, places, waiting, cores of one envelope), from poste --qui and the lock folder's coeurs-libres."""
    if out is None:
        return "-", "-", "-", "-"
    held = sum(1 for l in out.splitlines() if l.startswith("tient ") and len(l.split()) > 1 and l.split()[1] in CPU_PLACE)
    waiting = sum(1 for l in out.splitlines() if l.startswith("attend ") and len(l.split()) > 1 and l.split()[1] in CPU_PLACE)
    m = re.search(r"lourds : \d+/(\d+) places", out) or re.search(r"places réglées à (\d+)", out)
    places = int(m.group(1)) if m else None
    free = read(os.path.join(os.environ.get("POSTE_LOCK_DIR") or os.path.join(HOME, ".cache", "rag3weaver-build"), "coeurs-libres"))
    try:
        free = int((free or "2").split()[0])
    except ValueError:
        free = 2
    envelope = f"{(n_cores - free) / places:.1f}" if places else "-"
    return str(held), "-" if places is None else str(places), str(waiting), envelope


def gpu_jobs(out):
    """(held, waiting, Σ declared GiB) of the poste gpu jobs (« tient gpu 12G … », « tient gpu+cpu 12G … »)."""
    if out is None:
        return "-", "-", "-"
    held = waiting = declared = 0
    for l in out.splitlines():
        w = l.split()
        if len(w) < 3 or w[1] not in ("gpu", "gpu+cpu"):
            continue
        if w[0] == "tient":
            held += 1
            m = re.fullmatch(r"(\d+)G", w[2])
            declared += int(m.group(1)) if m else 0
        elif w[0] == "attend":
            waiting += 1
    return str(held), str(waiting), str(declared)


def gpus():
    out = []
    for card in sorted(glob.glob(os.path.join(SYS, "class", "drm", "card[0-9]*"))):
        name = os.path.basename(card)
        if not re.fullmatch(r"card\d+", name):
            continue
        dev = os.path.join(card, "device")
        busy = read(os.path.join(dev, "gpu_busy_percent"))
        if busy is None:
            continue
        used, total, gtt = (read(os.path.join(dev, f)) for f in ("mem_info_vram_used", "mem_info_vram_total", "mem_info_gtt_used"))
        s = f"{name}:{busy.strip()}%:{gib(int(used)) if used else '-'}/{gib(int(total)) if total else '-'}"
        if gtt:
            s += f":gtt {gib(int(gtt))}"
        out.append(s)
    return ",".join(out) or "-"


def rotate(path, now):
    """Drop the lines older than KEEP_DAYS, when the first line is."""
    try:
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
    except OSError:
        return
    limit = (now - datetime.timedelta(days=KEEP_DAYS)).isoformat(timespec="seconds")
    if len(lines) < 2 or lines[1].split("\t", 1)[0] >= limit:
        return
    kept = [lines[0]] + [l for l in lines[1:] if l.split("\t", 1)[0] >= limit]
    with open(path + ".new", "w", encoding="utf-8") as f:
        f.write("\n".join(kept) + "\n")
    os.replace(path + ".new", path)


def main():
    os.makedirs(DIR, exist_ok=True)
    path = os.path.join(DIR, "poste-load.tsv")
    now = datetime.datetime.fromisoformat(os.environ["POSTE_LOAD_NOW"]) if os.environ.get("POSTE_LOAD_NOW") else datetime.datetime.now()
    n = cores()
    m = meminfo()
    total, avail = m.get("MemTotal"), m.get("MemAvailable")
    st, sf = m.get("SwapTotal"), m.get("SwapFree")
    load = (read(os.path.join(PROC, "loadavg")) or "- ").split()[0]
    qui = poste_qui()
    row = [now.isoformat(timespec="seconds"), cpu_busy(os.path.join(DIR, "poste-load.state")), load, str(n),
           gib(total - avail if total is not None and avail is not None else None), gib(avail),
           gib(st - sf if st is not None and sf is not None else None), gib(st),
           pressure("memory", "some"), pressure("memory", "full"), pressure("cpu", "some"),
           *heavy(n, qui), gpus(), *gpu_jobs(qui)]
    # Columns changed since the file began: the old file is kept aside, a new one starts.
    try:
        with open(path, encoding="utf-8") as f:
            head = f.readline().rstrip("\n")
        if head and head != "\t".join(COLUMNS):
            os.replace(path, f"{path}.before-{now.strftime('%Y%m%d-%H%M%S')}")
    except OSError:
        pass
    rotate(path, now)
    new = not os.path.exists(path)
    with open(path, "a", encoding="utf-8") as f:
        if new:
            f.write("\t".join(COLUMNS) + "\n")
        f.write("\t".join(row) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
