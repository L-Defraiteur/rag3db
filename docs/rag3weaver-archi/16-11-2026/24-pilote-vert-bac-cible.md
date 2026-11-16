# Pilote VERT ciblé : les deux raccords Sandbox

État : préparation publiée avant tests CPU et avant toute exécution native.

Source exacte `7425375d66a64b0782672225c59516f048682b00`, revue Mémoire favorable.
Ticket : [le témoin documentaire attend un accord sans superviseur](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md).
Les deux vrais rouges sur `223b7f55cb502deffe85584898d8fd340fd48518` et le pilote
`72338e04d7b48549e092bf740f26cdf608c2e4b4` restent publiés dans `480604f8`.
Leur échéance historique 1197212.902100429/1197227.902100429 est expirée.
Aucun ancien GO ne se reporte. La préparation Git et des copies est autorisée ;
l'exécution native exige une nouvelle revue et un GO C7 exact.

## Borne proposée

Portée commune **600 s maximum**, entrée par le vrai poste 32 Gio/HIGH=max,
quota par place et jobs=min(8, POSTE_FILS), un seul target/session existant.
Borne extérieure **TERM 780 s + 15 s de grâce = 795 s au total**, file incluse,
chronomètre neuf seulement au futur lancement autorisé. Après poste la portée
prend min(600, plancher(soft_end-now)-15), refus si <=60. Le final prend au plus
120 s et reste sous la même échéance soft ; nouvelle vérification après les
hashes puis après ouverture du résultat, verdict écrit et retour concordants.

Justification : les deux compilations rouges récentes ont pris 36.96 et 31.73 s ;
leur déroulé commun jusqu'au final a pris environ 123 s. Les caps sont 20 s de
préflight, 20 s de garde, 180 s par compilation, quatre inventaires de 10 s,
90 s maximum de publication et deux exacts de 30 s. Ces plafonds s'intersectent
avec les 600 s ; ils ne sont ni additionnés ni renouvelés. Les 180 s extérieures
supplémentaires couvrent la file et le final. Aucune promesse de durée d'un bâti
invalide, aucun réessai automatique.

## Recette concrète

Le dossier `doc-bac-green/` reprend les primitives de la recette rouge sans
modifier `doc-bac-red/`. Préparer physiquement une archive détachée 223 et ses
administrations de gitlink, puis avance rapide du même W source vers 742.
Vérifier identité, inventaire complet T et ancien cache avant/après : aucun
Cargo, aucune garde réelle et aucune purge durant cette préparation.
Le contrôleur rouge 72338 reste figé ; un checkout de contrôle séparé porte le
nouveau pilote sur la même branche de lot.

Avant le futur premier Cargo : reçu 223 rouge terminé, deux ELF copiés intacts,
2111 preuves au manifeste épinglé, vieux contrôleurs propres ; scan frais après
poste exe/cwd/FD/argv/start/cgroup avec les trois rôles exacts et UNKNOWN attribué
bloquant. La vraie garde du même propriétaire doit rester silencieuse, code0,
et l'inventaire T rester strictement identique avant/après. SDK59 complet épinglé,
exporteur réel une fois par appel, environnement privé sans Vertex/clé/PG/modèle.

Deux builds Cargo JSON --lib et --test e2e, no-default, code/daemon/rag3db-native,
locked/offline/no-run. Exiger deux artefacts exécutables uniques, features exactes,
chemins sous le même T et préfixe source742 dans les octets ELF. Quatre inventaires
(--list et --ignored pour chaque ELF), noms présents et non ignorés. Ensuite ledger
et deux copies ELF réhashés sur PC et portable, publication explicite AVANT marker
runtime et avant les deux appels exacts. Pause bornée90 s, défaut = STOP.

Deux juges distincts / quatre invocations brutes : le parent de chaque ELF relance
le même nom dans le même ELF. Chacun doit imprimer deux résumés 1/0/0, les deux
bons noms, un marker parent positif et une attestation JSON enfant unique du
Sandbox réellement commité avant tâche : HEAD, égalité manifeste/déclaration et
HEAD/déclaration, commands=approval, Q privé extra_read, cible et TMP symboliques,
jobs1/offline. Les TempDir réussis disparaissent normalement ; aucune reconstruction
ni lecture de fichiers disparus pour prétendre avoir conservé une fixture verte.

Tout refus, panne de compilation, test ignoré, mauvais nom ou attestation absente
fixe le premier rouge immuable et arrête avant la suite. Verdict VERT uniquement
avec driver positif, portée0/restes[]/nettoye/bus absent, autorités terminées,
quiescence et source/ELF/cache ancien/SDK réattestés dans la même borne.
Qualification ciblée du bac uniquement : pas de Vertex, D6, F8 global, lib entière,
NAV, service utilisateur ou livraison master. Ticket et pas-va-au-bout restent ouverts.

## Témoins CPU prépubliés

`test_contracts.py` exerce le vrai classificateur positif et ses refus, le profil
privé/quota, le constructeur de commandes/politique, les bornes et le verdict après
ouverture tardive. `test_registration.py` lit les vrais Cargo.toml/main/Rust et
exécute les affectations AST du vrai driver ; aucun Cargo ou ELF natif.
À cette publication : AST8 parsés, aucun test CPU de ce nouveau pilote encore joué.
