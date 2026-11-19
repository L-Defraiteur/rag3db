#!/usr/bin/env python3
"""**The latency of a query while an index embeds.**

Design `docs/embarquement/19-11-2026/01-voie-courte-des-requetes.md`. A client only (`GET /sante`, `POST /embed`) :
one thread sends batches of 128 texts of 1 000 to 1 400 characters without pause, like an index ; another sends a
request of one text every 500 ms and notes its latency. It reports, over `--seconds`, the median, the 95th
percentile and the maximum of the queries, and how many index batches were served. JSON on stdout (or `--out`).

Never against a service in use : start a test service on a free port.

Usage :
  python3 scripts/embed_latency.py --address 127.0.0.1:7990 [--seconds 60] [--out latency.json]
"""
import argparse
import json
import random
import statistics
import threading
import time
import urllib.request
from pathlib import Path


def arguments():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--address", required=True)
    p.add_argument("--seconds", type=float, default=60.0)
    p.add_argument("--batch", type=int, default=128)
    p.add_argument("--every", type=float, default=0.5, help="seconds between two queries")
    p.add_argument("--timeout", type=float, default=120.0)
    p.add_argument("--seed", type=int, default=19)
    p.add_argument("--out", type=Path)
    return p.parse_args()


def post(address, texts, timeout):
    req = urllib.request.Request(f"http://{address}/embed", data=json.dumps({"texts": texts}).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read().decode())


def main():
    a = arguments()
    with urllib.request.urlopen(f"http://{a.address}/sante", timeout=a.timeout) as r:
        identity = json.loads(r.read().decode())
    rng = random.Random(a.seed)
    words = ["index", "vector", "chunk", "scope", "embed", "query", "batch", "token", "service", "catalog"]
    def text(n):
        return " ".join(rng.choice(words) for _ in range(n // 7))[:n]
    stop = threading.Event()
    batches = [0]

    def index():
        while not stop.is_set():
            post(a.address, [text(rng.randint(1000, 1400)) for _ in range(a.batch)], a.timeout)
            batches[0] += 1

    t = threading.Thread(target=index, daemon=True)
    t.start()
    time.sleep(2.0)
    latencies = []
    end = time.monotonic() + a.seconds
    while time.monotonic() < end:
        start = time.monotonic()
        post(a.address, ["où se déclare la fonction qui coupe un texte à son plafond ?"], a.timeout)
        latencies.append(time.monotonic() - start)
        time.sleep(max(0.0, a.every - (time.monotonic() - start)))
    stop.set()
    t.join(timeout=a.timeout)
    ms = sorted(x * 1000 for x in latencies)
    report = {
        "address": a.address,
        "model": identity.get("modele"),
        "executable": identity.get("executable"),
        "short_request_texts": identity.get("short_request_texts"),
        "queries": len(ms),
        "index_batches": batches[0],
        "median_ms": round(statistics.median(ms), 1),
        "p95_ms": round(ms[min(len(ms) - 1, int(0.95 * len(ms)))], 1),
        "max_ms": round(ms[-1], 1),
    }
    out = json.dumps(report, indent=1)
    if a.out:
        a.out.write_text(out + "\n")
    print(out)


if __name__ == "__main__":
    main()
