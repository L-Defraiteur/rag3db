# Core adapté arrêté sur un prérequis de contenu du corpus

Ticket porteur : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Allocation C7-LUCIVY-CORE-890-16nov, GO exact890/2fa59, admit3cde/preflight23dd,
noncee14 ; recette08. Une seule admission et aucun retry. Résultat **ROUGE et
profil incomplet**, pas une régression attribuée au correctif warnings.

## Résultat et arrêt

Compilation offline release réussie39,95s,62 ELF produits. Les inventaires
étaient faits target par target avant leur invocation ; **16/62 inventoriés**
avant arrêt. Le pilote intercale inventaire et exécution, contrairement à une
lecture « tous les inventaires puis tous les tests » de la recette08 : aucun
inventaire complet des62 ni verdict global ne peut être déduit de cette passe.

- Huit targets avec tests effectivement réussis :117passés au total.
- Six targets sans test par défaut exécuté : ils ne sont pas verts ; ils
  contribuent18 des25ignorés observés. Les sept autres ignorés appartiennent
  aux targets bench_sharding et test_check_dictionary.
- Un target HOLD visité sans exécution :bench_vs_tantivy. Les huit autres HOLD
  du profil ne sont pas atteints, toujours nonqualifiés.
- Puis `test_derived_in_ram::derived_in_ram_answers_like_the_files` :
  **0passé/1échec/0ignoré**, rc101,3,53s, arrêt immédiat après ce processus.
- Les46autres targets n’ont pas été inventoriés/exécutés ; doctests nonatteints.
  Les trois ciblés warnings du résultat antérieur n’ont pas été rejoués ici.

[Log du premier rouge](preuves/indexed-warning-core-red-890c86a/logs/15-tests.log) :
les300 fichiers réels sont chargés, les index de référence/candidats construits,
puis l’assertion de précondition échoue dans compare(), ligne170 :
`mutex_lock strict: the panel must find something`.
La comparaison des résultats entre régimes n’est pas atteinte pour cette requête.
Aucune assertion assouplie, aucun remplacement de corpus, aucun rejeu pour
obtenir une couleur différente.

## Preuve RAW bornée au sous-ensemble exact

Après STOP, vérification RO autorisée par C7 sur les **300 seuls fichiers**
déjà épinglés dans linux-selections.json. Chaque fichier rehashé conforme au
manifeste de préparation. **Zéro occurrence littérale `mutex_lock`, et zéro
en ignorant la casse ASCII, dans ce sous-ensemble**. Cela ne dit rien de sa
présence ou absence dans l’ensemble du noyau2.6.0. Aucun autre fichier du
corpus n’a été inspecté pour cette conclusion ; aucun moteur de test relancé.

[Preuve RAW par fichier](preuves/indexed-warning-core-red-890c86a/subset300-raw-evidence.json),
SHA cd10365404c784782daac7a93519682a96f9d90e32d021a79c792505e8fcb998.
[Identité du cas natif](preuves/indexed-warning-core-red-890c86a/native-case.json),
SHA a03b6222d74fa358377813b1174fd15f192b1b5c7905c04076760c22d1cb5230 :
source890, SHA du fichier de test, requête/panel exact, identité du sous-ensemble,
log/temps et ELF testé106fa398fcc24296c1d4b94198a150ed87917fac66e46531af5c617444fcf9a2.
Aucun snapshot7.2 ou golden d’une autre version n’a été substitué.

Le défaut est celui de la **recette d’admission** : taille, extensions, nombre,
provenance et identité avaient été vérifiés, mais pas les préconditions de
contenu imposées par le panel (référence nonvide). Le ticket est complété
AVANT toute correction envisagée. Une future admission doit établir la
compatibilité Source.dataset/PANEL sur des contenus épinglés **avant le test**,
pas simplement compter cinquante fichiers. Les autres panels dictionary et
positions restent nonjoués ; ne pas leur transférer une qualification.

Proposition actuelle : garder ce cas **HOLD** avec ce défaut de recette explicite.
Un choix d’autre sous-ensemble demanderait une conception/RO des préconditions
et une admission distinctes, avec invariants et contenus exacts publiés ; pas
une recherche opportuniste de corpus qui ferait passer le test. Aucun GO de
recomposition ou de nouvelle passe n’est implicite dans ce résultat.

## Provenance, ressources et fin propre

[Bruts et empreintes](preuves/indexed-warning-core-red-890c86a/SHA256SUMS).
CommonROUGE101/erreurnull/restes[]/nettoyétrue, scopevalidation-wbvzcmca,
autoritéTERMINE PID3528029/start119072861. END1192525.699990913 inchangé,
contrôle final1190873.393703287 avantEND. Common détenu absent ; relevé final
borné : aucun processus owned/attribué actif, **11UNKNOWN nonattribués**
conservés. Les12puis13UNKNOWN de préparation ne sont pas blanchis en zéro.
Aucune absence universelle/proc revendiquée.

Enveloppe réellement relue : cwdCommonQ, mémoire34359738368/highmax/swap0,
cpu.max400000/100000, jobs4. Le shell distant fish a refusé la garde shell
set-eu, puis a exécuté cdQ et la commande autorisée ; une seule autorité réelle,
aucune relance. L’appel SSH rend1, le test et le verdictCommon portent101.
Scratch vide à la sortie ; source890 propre, marqueurcréateurT inchangé.
Même target propriétaire, copies/archives et caches originaux conservés ; pas
de nettoyage du target, de retrait worktree ou de purge à partir de ce résultat.

**Limite du ledger** : les62 SHA ELF ont été publiés dans un ledger seulement
APRÈS l’arrêt. C’est une attestation tardive, **pas** une preuve de ledger global
avant tests. Les inventaires par target précèdent les invocations, ce qui est
un fait différent. Aucun verdict n’efface cette limite de protocole ; elle est
consignée au ticket avant toute future recette.

RO de préparation Banc publiée94a2cdeea091c0fe753255090efd893c971097df,
`docs/banc/16-11-2026/09-ro-paquet-core-adapte-lucivy.md` : favorablePRÉPA
seulement, n’inclut pas ce résultat d’exécution ni de rehashphysique par Banc.
Les anciens trois ciblés890 restent la seule qualification warnings acquise.
Core complet nonqualifié, Linux7.2/ULP et PR42 toujours HOLD ; pas de fermeture,
main, fusion, tag, publication ou nouvelle permission implicite.
