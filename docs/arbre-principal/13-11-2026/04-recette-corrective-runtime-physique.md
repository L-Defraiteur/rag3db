# Étape corrective runtime du raccord physique

Préparation pour revue ; aucun GO ni lancement runtime. Le pilote vit dans
`cx-wt-qualification-physique`, branche `cx-arbre-qualification-physique-r1`,
hors source applicative figée. Source EXACTE :
`4e5ae8a324f50313b74f70772eb2e92ee41fad6f`.

## Arrêt initial et portée

La première globale est ROUGE : le pilote exécutait lib depuis /var/tmp,
sans Cargo.toml, alors que le témoin de politique lit current_dir()/Cargo.toml.
Pas de stack publiée avant arrêt ; attribution statique et métadonnées, pas
de défaut production établi. F8 8/8, minimal et compilations sont conservés,
ainsi que le ledger SHA256
`5e98111915438ee3e1ab03e74cef5413ab36a35a9735c3f3529dfa4fc6809078`.
L’échéance initiale 896572.526453239 est expirée et ne sera pas prolongée.

C7 demande une étape corrective DISTINCTE de 3600 s après poste réel, reçu
neuf et bus privé unique, arrêt au premier rouge. Aucun Cargo, F8, minimal
ou rebuild dans cette étape. Les quatre ELF du ledger sont réattestés
(source, SHA, 223 gabarits) aux mêmes chemins et copiés dans les preuves.
Lib repart intégralement depuis le crate, comme Cargo ; client et superviseur
restent /var/tmp. Puis NAV40 et quiescence Chrome réelle, CLI5, PG36,
lectures27+CREATE exact, hôte13 ; publication14 et symboles12 sont des sous-ensembles lib,
pas des invocations supplémentaires. Les quatre suites TX sont aussi exécutées
avec leur inventaire exact préalable.
Les ignorés de lib restent comptés séparément. Les filets e2e sont distincts.

## Lanceurs et gardes

Après revue et GO exact sur le commit pilote ET la source applicative :

```bash
bash /home/lucied/.cache/rag3weaver-build/cx-wt-qualification-physique/docs/arbre-principal/13-11-2026/raccord-physique/lancer-complement-global.sh 4e5ae8a324f50313b74f70772eb2e92ee41fad6f
```

Exécution exclusivement luciepc. Le lanceur prend le vrai poste lourd avant
confiner_validation --delai 3600. MemoryMax32GiB, CPU quatre rôles, SDK29
strict, target unique conservé. Nouveau répertoire de preuve
`cx-passe-raccord-physique-union-complement-v1` : refus s’il existe déjà.
Le pilote exige source et contrôle propres/figés. Ledger neuf à publier avant
runtime, coordination effective BGE7879/card2 avant lectures. Aucune réserve
ou utilisation de ce service n’est faite par cette préparation.

run_e2e.sh compile toujours : `e2e-runtime.py` extrait ses huit fonctions
runtime suivies, vérifie leurs octets contre git show source4e, et remplace
uniquement la stanza compilation de jouer_les_suites par l’ELF canonique
réattesté. Un seul e2e et aucun « autre test » sont admis ; toute invocation
Cargo par confined est refusée. Aucun faux compiler-artifact ni prétention
d’avoir exécuté le script complet. Filets, état de suite, résidus, arrêt et
archive restent les primitives réelles ; leurs hashes avant/après sont archivés.
Le disque de ces suites est sous la preuve privée : aucune rotation de
l’ancien e2e-gardes partagé. Parents propres conservés pour revue.
CREATE reste DIRECT env -u/bash avec profil capturé, sans second EXPORT ;
assertions strictes de profil, ELF et maps maintenues.

## PG et verdict

Nouvelle PG `cx_phys_2191_20261113_u1c` : absence préalable, création unique,
identité/OID/propriétaire/commentaire contrôlés puis réattestés avant dump
-Fc hashé, zéro session. Aucun DROP/reset. Ancienne PG
`cx_phys_2191_20261113_u1` conservée ; OID et dump requis avant tout retrait.

Le verdict futur distinguera préparation antérieure verte, premier runtime
rouge et étape corrective ; aucune réussite rétroactive du premier run.
UNKNOWN fd/exe/environ/maps n’est pas observation négative d’accès GPU,
y compris si devices=[] pour un PID protégé. CG illisible reste rouge.
Quiescence exige aussi absence des owned vivants protégés. Les preuves P14
ciblées ne valent pas qualification globale. Les trois tickets produit et
le ticket pilote restent ouverts jusqu’à qualification/livraison effective.
