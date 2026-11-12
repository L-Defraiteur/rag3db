# Passe globale de la publication d'accord — recette figée

GO C7 après A et confinementcdd354e7 ; base rebasée surA+a25, branche neuve
cx-ed-accord-publication-2 publiée. La source exécutée sera le commit qui publie
cette recette, annoncé avant lancement ; production03ee5cdcf réappliquée sans conflit.
Ticket : docs/tickets/closed/2026-11-12-qualifier-la-publication-concurrente-d-un-accord.md.

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

## Inventaires publiés avant runtime

Source réellement compilée `6f5276b39bdd1803dbd80d40b5352c68612e3ad3`, F5VERT5/5 et minimalchecklibVERT37,09s.
Les trois compilations et leurs listes sont vertes, sans aucun test global de lib/NAV/e2e
exécuté à ce stade. Inventaire complet aux deux postes, SHA256 `acb5e0566601b2940b33dcd3f0b8adc6a1ed70ecfc193b194fb2e01e7b45d76b`.
Lib :2294 tests dont65 ignorés, **2229 succès attendus** ; NAV40/0ignoré ;
lectures27/0ignoré ; démonhôte13/0ignoré. Les7 publication_accord_tests sont présents.
Les noms complets de tous les tests, ignorés et sélection NAV sont dans inventaire.json ;
aucune exclusion modifiée ou déduite de l'ancien compte.

- large : `rag3weaver-b68c217725032750`, SHA256 `30b2f790bd0905842406d38fef772d3e74598aba55a6700e34ee4b89ff918e71`, source6f527/220gabarits vérifiés.
- nav : `rag3weaver-0ec6f5856d755044`, SHA256 `02ceeef1440e72427a78d6657481f99925afd07e9f475953de916fbdeed8a1af`, source6f527/220gabarits vérifiés.
- e2e : `e2e-7cefc6db1fbc2614`, SHA256 `c82048040271b5798081f5b6549d664c911586ebe14a8fdcfe87d7a6ca4d54a9`, source6f527/220gabarits vérifiés.

Le contrôle de reprise est le SHA de cet inventaire, pas une nouvelle autorité ni
une approbation humaine ; le reçu commun reste seul propriétaire du budget/bus.
Deadline inchangée 866610.959853126, première publication avant lib/NAV/lectures/démon.

## Recette corrective commune, revue143a21706 favorable

Archi relit source143 : helper lectures déplacé sans changement de comportement vers
common,3témoins inchangés ; index hôte conserve ValueHTTP brut, autres outils texte
inchangés, assertions index/usages/clone/retrait intactes. Diff src/templates vide
depuis6f527. Le doublon b229 est remplacé par le ticket canonique6bf ; proposition
sansespace abandonnée sans exécution.

Correction exécutée sur le commit qui publie cette recette, figé/propre avant entrée.
Noms/modules : F5check5montages, compilatione2eFJSON, trois helpers
`e2e_lectures::recu_index::`, consommateurslectures27 et e2e_demon_hote13, aucun
ignoré dans helpers/suites. Comparaison des noms complets du nouvel e2e avec l'inventaire
initial860 : aucune suppression/skip ; son SHA/source/220gabarits sera publié après
compilation AVANT les helpers/consommateurs, dans le même reste de budget.

Seules commandes justifiées par le helper devenu commun : F5≤2400, compilee2e≤1500,
helpers≤30, lectures et hôte≤120chacune (118runtime+2cleanup réservées). Toutes bornes
intersectent le reste GLOBAL initial, deadline866610.959853126 inchangée ; le timeout
120déjàconsommé et tout temps de préparation/attente comptent. Aucun nouveau6000.
Lib/NAV inchangés ne sont pas rejoués ; source des preuves conservée6f527.

Entrée : POSTE_MEM_MAX32G et vrai poste lourd autour de entrer.sh, qui calcule le reste
APRÈS acquisition et exec LA primitive commune sans service intermédiaire ni marqueur.
L'ascendance poste est conservée et vérifiée, une portée/bus/reçu pour la corrective.
Le nouveau reçu doit avoir une échéance≤deadlineinitiale ; toute erreur STOP.
Pas d'upgrade exclusif, pas de --build. Les quatre variables CPU, CARGO_MANIFEST_DIR,
LD_LIBRARY_PATH29 et chemins privés sont complets. Journaux EXACTEMENT sous
`globale/racine de journaux`, espace/Unicode/JSON préservés ; source/e2e JSON/hash
figés. Pas de nouveau modèle/service ni de changement produit. 7879existant seulement
pourlectures, prise/fin coordonnée et santé BGE1024/sparse avant/après. Hôte sans
SERVICE_SPARSE : le journal initial était déjà fini, pas de modèle pour masquer ce bug.

Scripts externes correctif-journal, conservés aux deux postes avant entrée :
- passe.py SHA256 `cbbc511c027ff58ae5a3e8bfe9f4d2cd8945ad65217f4b49231ff748f4158f35`.
- driver.sh SHA256 `0bd0d4c100dd74793fc6940d1a935582e13bf3a648a2d09281343d0ec847cd1b`.
- entrer.sh SHA256 `088e3b69afeda047d9bb9a25e7de28c0c240d7297ab89bcf9b9c01d3797ac1ba`.

Les trois ELF initiaux ont été archivés avant recompilation car le chemin du binaire
e2e peut être réutilisé par Cargo ; les rouges et la fixture initiale restent conservés.
Validation finale composée seulement après verdict corrective/primitive verts et
quiescence vérifiée ; pas de clairon dense prêt pour les3vecteurs explicitement dus.

## Inventaire corrective publié avant helpers/suites

Source `2a31b271689df34aab77cc6170e623229cb47be7`, F5montagesVERT5/5 et compilatione2eJSONverts.
Nouvel e2e `e2e-7cefc6db1fbc2614`, SHA256 `921167177138e8b2d721190fbd72db47db02d2c090e298822b9bad8c1d3556de`,
source2a31/220gabarits vérifiés. Liste intégrale identique aux860 noms initiaux,
helpers3 inchangés, lectures27/hôte13 ; pas de suppression ou saut.
Inventaire aux deux postes SHA256 `655e2372dd630e510683827e6c57087c31328a55369053d4e1318988e4af9004` ; publication
avant runtime des helpers puis suites. Reçu correctif deadline866608.908275966
≤initiale866610.959853126, pas de reset. Lib/NAV initiaux6f527 conservés,
nouvel e2e à source testonly2a31 identifié distinctement.
