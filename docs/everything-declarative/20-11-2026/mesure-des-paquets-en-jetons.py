import sys, pathlib
from tokenizers import Tokenizer
tok = Tokenizer.from_file(str(pathlib.Path.home()/".cache/rag3weaver/granite-278m/tokenizer.json"))
racine = pathlib.Path(sys.argv[1]); bornes = [int(x) for x in sys.argv[2].split(",")]
textes = []
for f in sorted(racine.rglob(sys.argv[3])):
    lignes = f.read_text(errors="replace").splitlines(keepends=True)
    # paquets de lignes entières, au plus B octets, glissants par 10 lignes
    textes.append(lignes)
for B in bornes:
    n = pire = au_dessus = 0; ratios = []
    for lignes in textes:
        i = 0
        while i < len(lignes):
            j, taille = i, 0
            while j < len(lignes) and taille + len(lignes[j].encode()) <= B:
                taille += len(lignes[j].encode()); j += 1
            if j == i: j = i + 1
            t = "".join(lignes[i:j]).rstrip()
            if t.strip():
                k = len(tok.encode(t, add_special_tokens=True).ids)
                n += 1; pire = max(pire, k); au_dessus += k > 512
                if len(t) > 200: ratios.append(k / len(t))
            i += 10
    ratios.sort()
    q = lambda p: ratios[min(len(ratios)-1, int(p*len(ratios)))]
    print(f"B={B}: {n} paquets, max {pire} jetons, {au_dessus} > 512 ; jetons/caractère médiane {q(.5):.3f} p99 {q(.99):.3f} p99.9 {q(.999):.3f} max {ratios[-1]:.3f}")
