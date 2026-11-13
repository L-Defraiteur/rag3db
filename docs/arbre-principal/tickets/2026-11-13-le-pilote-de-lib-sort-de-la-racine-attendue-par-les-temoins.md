# Le pilote de lib sort de la racine attendue par les témoins

- **État** : ouvert ; globale source4e arrêtée, correctif de recette en préparation.
- **Famille** : produit.
- **Gravité** : la recette fait échouer un témoin existant par précondition fausse.
- **Atteignable en service** : non, pilote privé ; aucun défaut production établi.
- **Touche rag3weaver** : non côté produit ; pilote et répertoire des tests.
- **Pour** : arbre-principal ; revue ED et orchestration.
- **Signalé par** : première globale du raccord physique, source4e5ae8a32.

Parent : [migration/raccord physique](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).

## Rouge et préconditions observées

Après F8 8/8, vrai minimal et4ELF canoniques compilés/inventoriés, lib interrompue
au premier FAILED de code_tools::tests::a_closed_policy_refuses_and_an_open_one_reads_outside_the_source.
Le corps de panique n'a pas été imprimé avant arrêt ; aucune stack inventée.

Le test1709–1717 lit current_dir()/Cargo.toml puis unwrap le cas policy ouverte.
Le pilote force /var/tmp : son Cargo.toml est ABSENT, celui du crate PRÉSENT
(sonde métadonnées PC). ED/C7 confirment le code test/policy inchangé de master.
Corriger cwd de la recette, pas modifier politique ni test pour masquer l'écart.

Preuves : cx-passe-raccord-physique-union-v1/lib.log et ledger
5e98111915438ee3e1ab03e74cef5413ab36a35a9735c3f3529dfa4fc6809078 ; superviseur
validation-lj2z7d9o ROUGE/code1/restes[]/nettoyé, autorité terminée, bus absent,
scan contexte vide. NAV/CLI/lectures/PG-TX runtime non lancés ; PG synthétique
créée conservée. Les verts de préparation ne sont pas un vert runtime global.

## Première proposition, désormais remplacée après expiration

Pilote publié dans branche/WT séparés, SOURCE de test4e figée et4ELF inchangés.
Reprendre runtime COMPLET lib depuis cwd crate (comme Cargo), puis suites encore
non jouées, sous NOUVEAU reçu borné à l'échéance ORIGINALE896572.526453239,
après poste réel. Pas de reset6000 ni réutilisation du reçu terminé.
Aucune recompilation ; comparaison SHA/source/223gabarits et ledger conservés.
Client/superviseur restent hors WT. Métadonnées de cette correction distinctes,
rouge premier conservé, verdict composite futur doit citer les deux portées.

Le script run_e2e.sh n'offre pas d'entrée ELF sans son cargo --no-run. La recette
complémentaire doit donc soit réutiliser exactement ses fonctions runtime et
gardes avec seule stanza compilation remplacée par le chemin ELF réattesté,
soit attendre arbitrage C7 ; aucun Cargo caché et aucune compilation simulée.
Un PG synthétique neuf distinct est proposé pour éviter d'inférer la propreté
de celui créé avant l'arrêt ; l'ancien reste intact jusqu'à dump/retrait coordonné.


## Reprise corrective distincte — décision C7 après expiration

L’échéance initiale 896572.526453239 est expirée : aucune reprise ni prolongation
de ce reçu. Le rouge global initial reste conservé. Source application EXACTE
4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF du ledger inchangés.
Pilote préparé hors WT source, étape corrective DISTINCTE de 3600 s après poste
réel, runtime complet avec cwd crate, sans Cargo/rebuild/F8/min déjà verts.
PG initiale cx_phys_2191_20261113_u1 conservée, identité/OID et dump requis avant
tout retrait ; nouvelle PG privée cx_phys_2191_20261113_u1c, absence préalable
et création unique contrôlées. Aucun DROP/reset. Revue et GO exact requis avant
lancement. Aucun résultat runtime global vert ; P14 ciblé seul demeure qualifié.
Les champs protégés UNKNOWN ne prouvent pas une absence d’accès GPU.
Voir [recette corrective](../13-11-2026/04-recette-corrective-runtime-physique.md).
