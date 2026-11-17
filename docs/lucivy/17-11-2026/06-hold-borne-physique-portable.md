# HOLD : le parent portable n'est pas physiquement borné

Signal C7 `452dd0ccfc`, [même ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Source8b, T3552 et ancien Q v1 restent figés/non consommés. Aucun Cargo ni
corpus/natif. Les avis favorables de préparation ne prouvaient pas cette borne.
**La commande04 est HOLD et ne doit pas être lancée.**

Le code export047111 démarre `local_deadline=now+300` après armement et lancement
PC. Cette échéance n'interrompt pas un fsync/hash/écriture/retour bloqué. Le
timeout PC ne borne pas le processus portable ; la garantie physique annoncée
sur les deux rôles est donc absente.

## Contre-exemple proposé AVANT correctif

`outils/test-fs-export-physical-bound.py` lance le CLI exact047111 depuis une
copie immuable dans un TempDir, avec son vrai argparse/préflight/hash/JSON et
son vrai appel fsync. PATH fournit un faux transport ssh, jamais exécuté sur
un hôte ; sitecustomize bloque uniquement le fsync du ledger local après son
écriture. Le faux ssh n'interprète ni shell ni code PC. Aucun ELF, Cargo,
corpus, service, réseau ou scan de processus n'est nécessaire.

Le témoin prend les **300 secondes réelles**, sans horloge virtuelle ni nouveau
flag/env de budget que l'ancien CLI ne reconnaîtrait pas. Une observation de
PID vivant après300s et une trace prouvant l'entrée dans fsync sont archivées.
À302s le harnais tue son groupe de processus privé, y compris le faux enfant
de lancement. Cette borne de secours indépendante n'est jamais appelée une
garantie du CLI. Les erreurs de setup/argument/transport ne sont pas le rouge
attendu : les traces arm/launch/ledger et l'entrée fsync sont exigées avant
l'assertion discriminante. Journal brut et observation sont gardés avant échec.

Source du témoin publiée avant tout correctif ; **non exécuté pour l'instant**.
La recette de ce seul témoin Python, après revue/GO spécifique, sera :

```text
python3 test-fs-export-physical-bound.py --exporter <copie exacte047111>
  --proof <dossier neuf de preuves local>
```

Aucun ancien préflight/Q n'est consommé. Avant exécution, figer source/commande/
SHA et dossier de preuve du témoin ; publier son journal après. Le rouge doit
être réellement observé avant correction de l'enveloppe portable.

## Correctif à préparer ensuite

Composer la primitive existante `timeout`, sans troisième primitive de relais.
L'entrée CLI portable devra être sous un watchdog physique couvrant lecture
préflight, lancement, I/O locales, ACK, copie finale et nettoyage des enfants.
Le PC recevra une **durée restante conservatrice**, amputée d'une réserve de
transport ; son END ne sera pas recréé à300. Aucune comparaison des horloges
absolues de deux hôtes. Si un appelant utilise l'ancienne entrée non surveillée,
refuser avant toute action distante. Retour/échec doivent aussi terminer les
enfants SSH locaux, pas seulement le chemin heureux.

Nouveaux pins et RO ciblée après le rouge, puis nouveau GO exact distinct.
Aucune correction d'admission FS ni relance des neuf verts mémoire anticipée.
