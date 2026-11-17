# Passation pour Claude — arrêt prioritaire du 17 novembre

Arrêt demandé par Lucie, transmis par C7 : quota. Tous les coupons non consommés
sont révoqués ; les anciens END ne sont pas réactivables. Aucun nouveau calcul,
test, GO, health, provider, master, tag ou nettoyage. Ce document prime les
recettes historiques : leurs commandes sont INACTIVES. Ne pas reprendre sur un
rappel automatique. Aucun secret consigné.

## Git et périmètre

WT : `/home/lucied/.cache/rag3weaver-build/cx-wt-archi-pas-va-au-bout-global-pilot-r2`.
Branche : `cx-archi-pas-va-au-bout-global-pilot-r2`.
Avant cette passation, HEAD et origin vérifiés identiques :
`84df224eda1877509f6b403cbe92c425ba38d185`, WT propre. Ce commit publie
le rapport67 et le ticket après le dernier rouge. Le commit de cette passation
sera communiqué à C7 après push ; aucun code WIP non poussé.
L'arbre principal de Lucie n'a pas été modifié par cette passation.

## Livré et limites

Validateur pur `extension/rag3weaver/scripts/execution_entry.py` gelé, SHA
`565b7f39494911b65ef121f19565450c70803c93b9ef6e62599d570707dbf218` :
51 tests verts sous coupon exact, rapport53 ; preuve deux hôtes
`~/.cache/rag3weaver-build/cx-archi-entry-pure-green-68ea-17nov-r1`, manifeste
`003ca193ffc17c0d54eb6366aee549ce54b5a07f5e5ddc5d7d9fb1d015a671b3`.

Protocole source documenté dans `execution-entry-native-protocol/` (rapports54–55).
Runner `extension/rag3weaver/scripts/execution_entry_runner.py` : orchestration
**MODELED / FIXTURE_ONLY**, pas de backend OS. LIVE et fermeture Common refusés.
CP ACK false ; admission globale false ; SDK non attesté. Callbacks non
interruptibles : contrôles avant/après, aucune garantie physique de terminaison.
Les primitives natives, publication CP réelle, adaptateur Common et helpers de
recette native ne sont pas livrés ni qualifiés. Aucun résultat modèle ne les vaut.

## Travail en cours, rouge conservé

Dernière source de témoin : `2df73d1dc523beecc8cfe67d07e837767150fd92`.
Runner SHA `cf3e7a41fc30eabd0ef5ff1b377efbf102ac14e3443d9796e3b3f19099cb4a0e` ;
test SHA `d9d61264000941cb312bdc9f72bcd944bb364bc7b6c97cde888f10eb7880adf9`.
31 méthodes écrites ; aucune suite31 jouée. Aucun GO24/28/30 consommé.

Coupon consommé UNE fois : C7-ENTRY-WAITRC-RED-2DF7-17nov60.
Rapport67 : 1 FAIL `None != 7`, 0 ERROR/skip, rc1 ; 30 autres non sélectionnés.
Le RAW bytearray est correctement refusé observation-types, mais ce refus efface
le rc7 valide déjà reçu. Aucun fix de ce défaut n'a été écrit.
Preuves neuf fichiers réhashées local et PC, 0 écart avant tout fix :
`~/.cache/rag3weaver-build/cx-archi-runner-waitrc-red-2df7-17nov-r1`, manifeste
`44df0cb57934e562c9cc0753764aacc72fd06ceb578f341da5e2d9365d210f45`.
Enveloppe 0.06427222304046154 s, START897942.398542487/END897942.46281471.

Anciens rouges jamais repeints : baseline rapport57 (b40a819), snapshot/hachage
rapport61 (b3ab49f), observations mutables rapport64 (b511358). Leurs archives
et pins restent les sources exactes. Les verts de l'ensemble runner restent
NON JOUÉS. Ticket partagé OUVERT :
`docs/orchestration/tickets/2026-11-17-les-images-executees-manquent-d-un-producteur-de-preuves-commun.md`.
Les réserves CP/LIVE restent indépendantes ; aucune levée implicite.

## Deux hôtes et ressources

PC : `lucied@luciepc`. Archives citées sous le même R sur les deux hôtes,
vérifiées avant cette passation ; aucune nouvelle vérification physique ici.
L'exécution CPU autorisée est terminée et archivée ; aucun PID/scope détenu à
arrêter connu pour ce lot. Aucun P/Q/S natif créé, aucune réservation poste lourd
prise, aucun Cargo, SDK, target ou service lancé par ce runner modèle.
Les anciens targets/services/ressources d'autres lots ne sont pas attribuables
à ce lot et n'ont pas été nettoyés ou inspectés. Aucun sondage santé effectué.

## Trois prochains pas — inactifs jusqu'à reprise explicite

1. Lire docs/README.md, les règles d'orchestration courantes et procédure push ;
   reprendre avec C7. Vérifications Git RO possibles : `git status --short --branch`
   puis `git rev-parse HEAD origin/cx-archi-pas-va-au-bout-global-pilot-r2` dans
   le WT ci-dessus. Faire relire la preuve67 sans la rejouer.
2. Après reprise autorisée, préparer le correctif minimal : valider kind/code
   immuables et conserver rc avant la copie complète du wait, sans callback ni
   clock intermédiaire ; garder le refus du RAW malformé et l'identité des
   handles opaques. Publier source/pins31/rapport et obtenir RO totale.
3. Obtenir un **nouveau GO exact** avant toute importation, analyse exécutable ou
   suite31. Commande future seulement, actuellement INTERDITE : argv épinglé
   timeout TERM58/KILL2, Python3.14 -B, unittest -v -f test_runner_effects,
   cwd protocole et PYTHONPATH scripts. Aucun ancien coupon, budget ou GO natif
   ne vaut autorisation. Premier rouge STOP, 0 retry, preuves deux hôtes.
