"""Mesure de WRITE_PATIENCE (src/host_log.rs) : le temps que met une sortie à reprendre
après EAGAIN, écrite comme le relais l'écrit (non bloquante, poll POLLOUT).
Cibles : le socket de flux de journald (ce que systemd donne au stderr d'un
service) et un fichier en ajout (StandardError=append:). Rafales : 1 000 lignes
de 1 Kio, 10 lignes de 64 Kio ; trois passes chacune.

Usage : python3 -I scripts/mesure_patience_du_journal.py (écrit ~1,6 Mo de lignes
x dans le journal sous l'identifiant hooks-essai-mesure, et un fichier
momentané sous ~/.cache/rag3weaver-build)."""
import os, select, socket, sys, time, statistics

def burst(fd, n, size):
    line = b"x" * (size - 1) + b"\n"
    waits, writes = [], []
    for _ in range(n):
        done = 0
        while done < len(line):
            t = time.perf_counter()
            try:
                w = os.write(fd, line[done:])
                writes.append(time.perf_counter() - t)
                done += w
            except BlockingIOError:
                t0 = time.perf_counter()
                p = select.poll(); p.register(fd, select.POLLOUT)
                p.poll(5000)
                waits.append(time.perf_counter() - t0)
    return waits, writes

def journald():
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect("/run/systemd/journal/stdout")
    s.sendall(b"hooks-essai-mesure\n\n6\n0\n0\n0\n0\n")
    s.setblocking(False)
    return s, s.fileno()

def fichier():
    chemin = os.path.expanduser("~/.cache/rag3weaver-build/mesure-patience.log")
    fd = os.open(chemin, os.O_WRONLY | os.O_CREAT | os.O_APPEND | os.O_NONBLOCK, 0o644)
    return chemin, fd

def ms(x): return f"{x*1000:.2f}"

def rapport(nom, waits, writes):
    if waits:
        q = sorted(waits)
        p99 = q[min(len(q)-1, int(len(q)*0.99))]
        print(f"  {nom}: {len(waits)} EAGAIN ; attente médiane {ms(statistics.median(q))} ms, p99 {ms(p99)} ms, max {ms(q[-1])} ms ; écriture max {ms(max(writes))} ms")
    else:
        print(f"  {nom}: aucun EAGAIN ; écriture médiane {ms(statistics.median(writes))} ms, max {ms(max(writes))} ms")

print(f"charge : {os.getloadavg()}")
for cible in ("journald", "fichier"):
    for (n, size) in ((1000, 1024), (10, 64 * 1024)):
        for passe in range(3):
            if cible == "journald":
                obj, fd = journald()
            else:
                obj, fd = fichier()
            waits, writes = burst(fd, n, size)
            rapport(f"{cible} {n}×{size//1024} Kio passe {passe+1}", waits, writes)
            if cible == "journald": obj.close()
            else: os.close(fd); os.unlink(obj)
