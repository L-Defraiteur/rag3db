# Raccord de pin de fixture uniquement, aucun rejeu

Cadrage C7 a030139580, après STOP CA1C et conservation des seize fichiers aux
deux hôtes (manifeste41c737…74b93), rapport05/33cfbed886. Le paquet CA1C reste
figé et son FAIL de montage n'est pas requalifié après coup.

Le seul changement de Python est dans le helper gate_fixture de
`test_publication_guard_counterexamples.py` : après calcul du SHA réel du
prepared, `base.gate.pin = pin`. Ce même SHA était déjà retourné au test et
écrit dans bootstrap. Les dix corps de tests, les assertions SHA du vrai Gate,
le moteur558, les wrappers, le pilote et les fixtures historiques restent
inchangés. Pas de suppression de contrôle ni de noyau de substitution.

Le fichier groupé est inchangé : trois anciens cas et sept compléments, dix
sélections exactes. Le helper commun a une nouvelle empreinte, contrairement
au paquet précédent ; la note04 décrit la prépublication CA1C historique.
Les nouveaux pins identifient le nouveau helper, le cwd propre à cette branche,
le groupe et le moteur inchangé. Aucun import/parser/test/OS effectué.

RO ciblées Mémoire/TTY avant un NOUVEAU GO CPU distinct. Rien n'est rejoué avec
le GO CA1C consommé. La prévision dix FAIL causaux demeure non observée. Les
conditions de préparation du correctif moteur restent non acquises tant que
ce résultat précis n'est pas observé et conservé sous une autorisation neuve.
Les neuf motifs précédemment observés restent dans les archives d'origine.

La nuance TTY première cause RPC est conservée au ticket : le seul délai0,4s et
un TimeoutError générique ne prouvent pas l'origine de la cause. Tout futur
vert revendiquant cette propriété devra avoir son témoin causal renforcé.
