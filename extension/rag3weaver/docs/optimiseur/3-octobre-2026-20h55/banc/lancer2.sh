#!/bin/bash
# Mesures complémentaires : JevK5 en taille 2B (même banc), puis critère en français et montage à deux étages
# pour les deux tailles. CPU seulement, huit fils.
D=/home/lucied/.cache/rag3weaver-build/decision
export HF_HOME=$D/hf TMPDIR=$D/tmp CUDA_VISIBLE_DEVICES="" TOKENIZERS_PARALLELISM=false OMP_NUM_THREADS=8
source $D/venv/bin/activate
cd $D/banc
python - <<'PY'
from huggingface_hub import hf_hub_download
import os
p = hf_hub_download("alibiserikbay/JevK5-GGUF", "jevk5-2b-v0.2-Q8_0.gguf")
print("JevK5 2B ->", round(os.path.getsize(p) / 1e6), "Mo")
PY
export HF_HUB_OFFLINE=1
# banc.py : accepter l'étiquette jevk5-2b (température de la carte : 1,42)
python - <<'PY'
p = "banc.py"; s = open(p).read()
a = 'elif modele == "jevk5":'
b = 'elif modele in ("jevk5", "jevk5-2b"):'
if a in s:
    s = s.replace(a, b)
    a2 = 'client = JevK5GGUF(url="http://127.0.0.1:8091", temperature=1.367)  # température de la carte pour la v0.3 4B'
    b2 = '# températures données par la carte du modèle : 1,367 pour la v0.3 4B, 1,42 pour la 2B\n    client = JevK5GGUF(url="http://127.0.0.1:8091", temperature=1.42 if modele == "jevk5-2b" else 1.367)'
    assert a2 in s
    s = s.replace(a2, b2)
    open(p, "w").write(s)
PY
serveur() { # fichier gguf
  llama-server -m "$1" -c 8192 -ngl 0 --device none -t 8 --host 127.0.0.1 --port 8091 > $D/banc/llama-server.log 2>&1 &
  SRV=$!
  for i in $(seq 1 120); do curl -s -m 2 http://127.0.0.1:8091/health | grep -q ok && break; sleep 1; done
}
arret() { kill -TERM $SRV; wait $SRV 2>/dev/null; }
S=$D/hf/hub/models--alibiserikbay--JevK5-GGUF/snapshots
echo "================ JevK5 2B"
serveur "$(ls $S/*/jevk5-2b-v0.2-Q8_0.gguf | head -1)"
python banc.py jevk5-2b 2>&1 | tail -6 | cut -c1-200
python banc2.py jevk5-2b 2>&1 | grep -v -i "warn\|Loading weights" | tail -8 | cut -c1-260
arret
echo "================ JevK5 4B"
serveur "$(ls $S/*/jevk5-4b-v0.3-Q4_K_M.gguf | head -1)"
python banc2.py jevk5 2>&1 | grep -v -i "warn\|Loading weights" | tail -8 | cut -c1-260
arret
echo "serveurs arrêtés : $(pidof llama-server | wc -w) en vie"
