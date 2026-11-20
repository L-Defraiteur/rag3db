"""chunk_lines of src/chunker.rs (master, 20 November 2026), ported : cores of max_lines lines or max_size bytes,
then overlap_lines lines of context on each side, each side at most max_size bytes ; a line longer than max_size is
cut into cores without context ; a last core under 50 bytes joins the previous one."""
import sys, pathlib, statistics
from tokenizers import Tokenizer
def chunk_lines(text, max_size, max_lines=30, overlap=5):
    b = text.encode()
    debuts = [0] + [i + 1 for i, c in enumerate(b) if c == 10 and i + 1 < len(b)]
    n = len(debuts); fin = lambda l: debuts[l + 1] if l + 1 < n else len(b)
    cores = []; l = 0
    while l < n:
        if fin(l) - debuts[l] > max_size:
            s = debuts[l]
            while s < fin(l):
                e = min(s + max_size, fin(l)); cores.append((l, l + 1, s, e, True)); s = e
            l += 1; continue
        f = l + 1
        while f < n and f - l < max_lines and fin(f) - debuts[l] <= max_size: f += 1
        cores.append((l, f, debuts[l], fin(f - 1), False)); l = f
    if len(cores) >= 2:
        d, f, _, lb, one = cores[-1]
        if not one and not cores[-2][4] and lb - debuts[d] < 50:
            cores.pop(); p = cores[-1]; cores[-1] = (p[0], f, p[2], lb, p[4])
    out = []
    for d, f, cs, ce, one in cores:
        if one: sb, eb = cs, ce
        else:
            sl = d
            while sl > max(d - overlap, 0) and debuts[d] - debuts[sl - 1] <= max_size: sl -= 1
            el = f
            while el < min(f + overlap, n) and fin(el) - fin(f - 1) <= max_size: el += 1
            sb, eb = debuts[sl], fin(el - 1)
        t = b[sb:eb].decode(errors="replace").rstrip()
        if t.strip(): out.append(t)
    return out
if __name__ == "__main__":
    tok = Tokenizer.from_file("/home/lucied/.cache/rag3weaver/granite-278m/tokenizer.json"); tok.no_truncation(); tok.no_padding()
    texts = [f.read_text(errors="replace") for f in sorted(pathlib.Path(sys.argv[1]).rglob("*.rs"))]
    for size in [int(s) for s in sys.argv[2].split(",")]:
        ks = [len(tok.encode(c).ids) for t in texts for c in chunk_lines(t, size)]
        over = sum(k > 512 for k in ks)
        print(f"max_size {size} : {len(ks)} morceaux, {over} au-dessus de 512 ({100*over/len(ks):.1f} %), médiane {statistics.median(ks):.0f}, p95 {sorted(ks)[int(.95*len(ks))]}, max {max(ks)}")
