# Issue 01 — quatorze minutes pour trente fichiers

**6 septembre 2026, 15h–16h.** La suite cloud a mis 862 s à ingérer
`src/dataflow` avec BGE-M3. Lucie : *« ça devrait pas mettre des années pour
indexer si peu ? y a pas optim à faire dans le chemin d'indexation ? »*. Ce
document est la mesure, dans l'ordre où elle a été faite, parce que la première
explication était fausse.

## Ce qu'on indexe

| | |
|---|---|
| fichiers | 30 (`src/dataflow`), 1,14 Mo |
| scopes | 1 633, chacun avec **tout son texte source** ; l'imbrication (module → impl → fn) fait 1,5 fois la source à embarquer : 1,75 Mo |
| chunks | 5 396 (découpe sémantique, 1 000 caractères, recouvrement 100) |
| modèle | BGE-M3 sur burn/wgpu (radv), fp32, par le démon `rag3weaver-embeddings` |

## Les mesures, dans l'ordre

`tests/e2e_mesure_ingestion_code.rs`, `RAG3WEAVER_INGEST_PROFILE=1`, seul sur
le poste, carte libre (32 Go).

| passage | temps d'embarquement | ce qui a changé |
|---|---|---|
| suite cloud, deux tests en parallèle | 862 s | — |
| mesure seule, démon **de la veille** | 450 s | le régime `confort` corrigé côté client ne change rien : le démon, lancé le 5 à 15h55 avec l'ancien code, redécoupe en 2 048 et dort 40 % |
| démon relancé (code du jour, debug) | 354 s | GPU à 95 % : c'est le calcul lui-même |
| démon en release | 337 s | le debug n'y était pour rien |
| **lots triés par longueur** | **149 s** | le tokenizer rembourre au plus long du lot ; dans l'ordre d'arrivée un lot mêle 50 et 1 000 caractères, la carte calcule 1,75 fois le texte (simulé), et l'attention est quadratique |

**Six fois moins** que le point de départ, sans toucher au modèle.

## Ce qui a été faux avant d'être mesuré

1. « C'est le régime `confort` » — vrai en partie, mais le client corrigé n'a
   rien changé parce que **le démon vivait encore avec l'ancien code**. Un démon
   survit à celui qui l'a lancé, c'est son rôle ; il survit donc aux
   reconstructions, et le harnais réutilisait n'importe quel démon qui répondait
   avec la bonne identité. Corrigé : l'identité porte l'empreinte de
   l'exécutable (`chemin@mtime`), `POST /quitter` existe (refusé si exposé), et
   le harnais remplace un démon d'une autre construction.
2. « La rafale courte est un budget mémoire » (knowledge dump du matin) — pas
   mesuré, et faux sur ce poste : deux cartes de 32 Go, le cgroup de 16 Go
   borne la mémoire hôte du build.

## Ce qui reste, et à qui

| reste | mesure | à décider |
|---|---|---|
| l'imbrication embarque 1,5 fois la source | scopes emboîtés, chacun avec son texte entier | un scope pourrait embarquer son texte **moins celui de ses enfants** (signature, docstring, corps propre). Question de fond sur l'index de code, pas de ce chantier |
| le modèle en fp32 sur wgpu | ~15 chunks/s après tri, GPU à 88–95 % | fp16, ou un backend natif ; c'est le chantier burn, pas rag3weaver |
| la simulation compte 3 532 chunks, la base 5 396 | l'écart vient de la découpe réelle (titre + contenu, recouvrement) | à regarder si on veut la simulation exacte ; elle a suffi à prédire le gain |
| deux tests cloud en parallèle sur un démon | 862 s contre 450 s | la suite cloud pourrait ingérer moins (les missions touchent deux fichiers) |
