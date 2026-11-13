# Coût CPU de la barrière : mesure terminée, pas un verdict de livraison

Ticket unique : `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.
Porteur Lucivy ; relecture et arbitrage C7/ED. Aucun seuil inventé.

Le correctif baisse le débit dans sept cas sur huit de ce microbanc ; un cas
borné à quatre producteurs gagne. Cela invite à revoir le coût du mécanisme
avant intégration, sans conclure sur une ingestion réelle ni sur le mutex seul.
Le delta global inclut admission, Arc et HandlerGuard.

## Résultats

Débits médians en millions de messages/s ; six échantillons par variante et
par cas, trois blocs ABBA. Capacité0 signifie non borné,1024 signifie borné.
Échauffement10000 puis100000 messages par producteur ; compte Fence exact.

| Dispatch | Producteurs | Capacité | A ancien | B barrière | Variation B/A |
|---|---:|---:|---:|---:|---:|
| worker | 1 | 0 | 4.770 | 3.296 | -30.9 % |
| worker | 1 | 1024 | 4.149 | 3.424 | -17.5 % |
| worker | 4 | 0 | 5.734 | 3.436 | -40.1 % |
| worker | 4 | 1024 | 1.922 | 3.459 | +80.0 % |
| pinned | 1 | 0 | 2.727 | 2.173 | -20.3 % |
| pinned | 1 | 1024 | 2.746 | 2.172 | -20.9 % |
| pinned | 4 | 0 | 2.684 | 2.434 | -9.3 % |
| pinned | 4 | 1024 | 2.681 | 2.386 | -11.0 % |

Dispersion/min/max/quartiles dans [comparaison brute](preuves/mesure-envois-fffe/comparaison.json),
[96échantillons](preuves/mesure-envois-fffe/samples.jsonl) et identités/raw de
chaque cas. Aucun compte perdu :96/96 valides ; aucun replay.

## Sources, conditions et portée

A663975adf11fc4a9d6cadf90e70ee68f6c9db075 ; Bfbd3f0a4179917e3674fa52a56ffde027831a354.
Pilote/harness issus exactement de fffe6b74ab86eb121f1a17014660fd63b7a2fd78.
Sources propres avant/après ; mêmes octets Rust A/B et locks identiques vérifiés.
Release/debug0, j4, MemoryMax8GiB/HIGHmax, affinité20,21,22,23 vérifiée.
Vrai posteMESURE exclusif, aucun lourd tenu pendant le créneau ; poste rendu
immédiatement à fin. Deux builds inclus dans600s global, exclus des chronos.
Fence incluse dans le chrono, Shutdown et notification réelle de Drop horschrono.
Rawstdout fichier ouvert avantPopen, identité persistée avantwait, hookpanic
identique A/B ; aucun cas en échec, aucun firstred créé.

Pinned émet dans les deux sources un diagnostic par envoi sans notifier :
stderr vers /dev/null identique, mais formatage/syscalls demeurent dans le
coût. Les cas pinned peuvent donc masquer une partie du delta ; ils ne mesurent
pas une ingestion sans diagnostics. Aucune simulation de stockage/SDK/DB/GPU.

## Preuves et suite

[Manifeste des preuves](preuves/mesure-envois-fffe/SHA256SUMS),
[métadonnées/sources/locks/exécutables](preuves/mesure-envois-fffe/metadata.json),
[verdict du confinement](preuves/mesure-envois-fffe/confinement-verdict.json),
[quiescence](preuves/mesure-envois-fffe/quiescence.json).
ConfineurVERT/code0/erreur=null/restes[]/nettoye=true, autoritéterminée et scan
PID/start vide ; END919800.434295504, contrôlefinal919306.237361605,494srestants.

Le vert fonctionnel179+6 reste acquis pour sa source ; cette mesure ne le
remplace pas et ne ferme pas le ticket CI ni n’attribue le holderD17.
Reste : relecture des bruts et arbitrage du coût ; si une autre primitive est
préparée, elle doit conserver les garanties puis être qualifiée sur sa source
exacte avant toute comparaison nouvelle. Aucune nouvelle variante, fusion,
version, release ou publication engagée par cette mesure.
