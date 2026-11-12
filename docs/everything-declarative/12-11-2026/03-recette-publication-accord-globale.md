# Passe globale de la publication d'accord — recette figée

GO C7 après A et confinementcdd354e7 ; base rebasée surA+a25, branche neuve
cx-ed-accord-publication-2 publiée. La source exécutée sera le commit qui publie
cette recette, annoncé avant lancement ; production03ee5cdcf réappliquée sans conflit.
Ticket : docs/rag3weaver-archi/tickets/2026-11-12-qualifier-la-publication-concurrente-d-un-accord.md.

Recette externe aux deux postes I-luciepc/accord-publication-20261112/globale :

- passe.py SHA256 `137dce22b34e602b15bee3cc1fb74acd2f456bf6466ba81d704ee48a75ad9bf9` ;
- driver.sh SHA256 `b69060d849e17f601ad03b8f946702eb0250d2fd366a35ebfa7bae21db3239b8`.

Commande canonique, depuis HOME, source W figée et propre :

```bash
POSTE_MEM_MAX=32G python3 "$W/extension/rag3weaver/scripts/confiner_validation.py" lancer --delai 6000 --preuves "$O/supervision" -- bash "$O/driver.sh" "$SOURCE"
```

Aucun outerservice/marker-seul/wrapperbusparChrome. LA primitive acquiert le poste lourd
réel avant le budget, MemoryMax32/Swap4, un bus sans activation, reçu actif et nettoyage
attribué. Le driver charge l'échéance du reçu ; toutes les bornes en sont le minimum,
aucun reset. BUILD_MEMORY_HIGH=0 ; memory.high=max vérifié. Pas de --build, pas d'upgrade
exclusif imbriqué ; une mesure en attente garde sa priorité selon poste.

F5check≤2400, vrai check --locked --lib --no-default-features≤120, compilation/inventaires
des trois ELF≤1500 au total, liblarge≤900, NAV≤600, lectures≤120, démonhôte≤120.
Premier rouge/timeout/écart d'observateur STOP, pas de retry inchangé ; une commande
laissée vivante sera arrêtée par la primitive sur sortie rouge du driver. Target32Go
surveillé pendant les commandes. Lib29 complète épinglée, -j8/incremental0.

CARGO_MANIFEST_DIR explicite et LD_LIBRARY_PATH du magasin29 pour les exécutables directs.
F=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm ;
NAV ajoute navigateur. Les trois compilations --no-run --message-format=json désignent
exactement leurs ELF, puis --list/--ignored --list sont conservés avant tout runtime.
Sept tests publication_accord_tests doivent être présents ; sélection NAV40 non ignorés,
lectures27/démonhôte13 incluent tous les tests de ces suites. Le compte large sera celui
du binaire exact, jamais un nombre dérivé de l'ancien master. Pause de publication des
inventaires et SHA avant runtime, dans le même6000 ; reprise par concordance de leur SHA
avec l'artefact inventaire-publie, sans modifier les exclusions ni reconstruire.

Quatre variables Burn CPU explicites, pas de modèle local/inférence lancé. Énumération
Vulkan possible et autorisée, RADV connu conservé ; pas de promesse zéroFD/pilote.
XDG/state/cache/runs/chats/tâches privés ; journaux sous « racine de journaux » avec espace,
sans segment commandes. BasePGneuve cx_ed_accord_publication_20261112 créée avec refus
si préexistante ; current_database vérifié et URL explicite pour les unités PostgreSQL
de la lib large. Aucune base/configuration utilisateur. Dump+SHA avant son retrait final.

Lectures nécessite le creux BGE réel : service7879 existant autorisé par C7, recherche
confirme aucun usage ; mémoire informée. Pause de coordination avant cette phase, puis
santé avant/après exige bge-m3/dim1024/facticefalse/sparse=true. Seule cette phase reçoit
SERVICE_SPARSE=127.0.0.1:7879 ; pas de nouveau service/modèle ni usage siège1. Annoncer
prise/fin ; NAV emploie le Chromium natif direct et hérite du bus unique.

Observateur des trois ELF en runtime : PID/starttime/mapsnative29/cgroupMax32/source
embarquée/220gabarits/SHA ; observateur de la primitive pour tous les descendants.
Un verdict-driver vert sera composé seulement avec le verdict final vert de la primitive,
quiescence des PID/profils/bus comprise. Preuves deux postes, puis revue et tourmaster.
