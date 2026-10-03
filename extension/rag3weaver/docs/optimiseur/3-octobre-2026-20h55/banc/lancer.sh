#!/bin/bash
# Lance le banc pour les cinq modèles, l'un après l'autre, sur CPU (huit fils), sans toucher à la carte.
D=/home/lucied/.cache/rag3weaver-build/decision
export HF_HOME=$D/hf TMPDIR=$D/tmp CUDA_VISIBLE_DEVICES="" TOKENIZERS_PARALLELISM=false OMP_NUM_THREADS=8 HF_HUB_OFFLINE=1
source $D/venv/bin/activate
cd $D/banc
for m in granite reranker gliner laya; do
  echo "================ $m"
  python banc.py $m 2>&1 | grep -v -i "warn\|Fetching\|You are using a model of type\|self.encoder" | tail -8 | cut -c1-200
done
echo "================ jevk5 (llama-server sur CPU)"
G=$(ls $D/hf/hub/models--alibiserikbay--JevK5-GGUF/snapshots/*/jevk5-4b-v0.3-Q4_K_M.gguf | head -1)
llama-server -m "$G" -c 8192 -ngl 0 --device none -t 8 --host 127.0.0.1 --port 8091 > $D/banc/llama-server.log 2>&1 &
SRV=$!
for i in $(seq 1 120); do curl -s -m 2 http://127.0.0.1:8091/health | grep -q ok && break; sleep 1; done
curl -s -m 2 http://127.0.0.1:8091/health | cut -c1-80; echo
grep -i -m3 "offload\|vulkan\|device\|n_gpu_layers" $D/banc/llama-server.log | cut -c1-160
python banc.py jevk5 2>&1 | tail -8 | cut -c1-200
kill -TERM $SRV; wait $SRV 2>/dev/null
echo "serveur arrêté"
ls -la $D/banc/resultats/
