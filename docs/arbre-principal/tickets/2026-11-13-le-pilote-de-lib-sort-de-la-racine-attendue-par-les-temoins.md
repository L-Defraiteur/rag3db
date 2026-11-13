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


### Delta de garde CREATE après revue 9587eac0a

Recherche identifie une assertion garantie : le runtime e2e utilise la racine
privée preuve/runtime-disk, mais CREATE exigeait encore la racine historique
partagée. La garde exige maintenant exactement o/runtime-disk/e2e-PID/e2e_lectures.
Contraintes PID, TMP, checkpoints, cache, état possédé et absence de résidus
inchangées. Aucun runtime exécuté ; nouvelle revue exacte avant GO.


## Étape corrective distincte : arrêt au premier nouveau rouge

C7 a donné GO exact au pilote e8721a5683166e8ff688e55977bc79cdcf50de11,
application figée 4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF hérités.
Portée DISTINCTE de 3600 s : départ monotone 898110.830219227, échéance
901710.830219227 ; aucune prolongation/réinitialisation de l’ancienne portée.
Aucun Cargo/F8/min/rebuild exécuté. Ledger neuf publié et quatre ELF copiés
avec SHA vérifiés AVANT runtime :
7fe4942919dd2ff3fede289c010be11509b5b1fb026b61c7c1e704bf5b9d209a.

Lib depuis le crate : ancien juge CWD passé ; nouveau premier FAILED :
daemon_host::atelier_fichiers::tests::the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree.
Arrêt immédiat, durée portée 85.917 s, runtime environ 8 s. Aucun résumé
lib complet, aucun corps de panique publié avant arrêt : cause précise NON
attribuée. Ni rejeu inchangé ni Cargo pour récupérer une panique. NAV/CLI/
lectures/BGE/PG-suites/TX non lancés. Le rouge global reste ROUGE.

Superviseur validation-ej1zgwoe : ROUGE/code1/restes[]/nettoyé ; bus
/tmp/r3v-py3oi02g absent. Scan RO exe/cwd target+deux WT : live[].
Les UNKNOWN restent non concluants sur absence GPU. PG u1 conservée et
PG u1c créée une seule fois, OID1989865 ; aucune suite PG jouée, aucun DROP.
Fixtures restantes conservées, sans ouverture native ni nettoyage.

Preuves PC : cx-passe-raccord-physique-union-complement-v1 ; copie locale
cx-preuves-raccord-physique-union-complement-v1-local/client : 2843 fichiers
identiques SHA au manifeste après arrêt. Superviseur aussi copié localement.
SHA lib.log : 8295e630e40de1f29d5b0a7df6eae5643cdb865e69722a66ce004acbbf3b71a5.
C7/ED poursuivent attribution RO ; aucun défaut production encore établi.
P14 ciblé reste vert dans sa portée ; aucune qualification globale obtenue.


Diagnostic RO tampon : graphe conservé d’une fixture concurrente prouve un
refus IndexNode du profil commun512MiB contre minimum de garde1GiB. Identité
du juge FAILED non établie par ce graphe ; source estimate.rs64–72 et
index_nodes.rs714–718 corroborent le contrat. Voir rapport05 pour preuve
et limites. Aucun changement de garde/produit et aucun rejeu.


## Préparation v2 profil 1 Gio — échéance corrective conservée

Décision C7 : nouveau pilote hors sources figées, profil global
RAG3DB_BUFFER_POOL_SIZE=1073741824 selon garde IndexNode/BufferCheck observée.
Aucune modification produit/garde ; pas de minimum universel moteur revendiqué.
Trace open_branch concurrente et contrat prouvent un défaut de profil, sans
inventer l’identité du FAILED. La lib entière et toutes les suites sont à
qualifier de nouveau, mêmes quatre ELF app4e, aucun Cargo/F8/min/rebuild.

Nouvelle branche cx-arbre-qualification-physique-r2, preuve complement-v2,
PG u1d créée seulement après absence préalable ; PG u1/u1c conservées.
Après vrai poste, reçu neuf avec délai floor(901710.830219227−monotonic)−3 ;
refus si reste<=60. Driver coupe chaque phase au minimum de son cap et
de cette échéance (marge15s), finaliseur vérifie échéance effective ET
borne absolue. Départ initial correctif898110.830219227 conservé : aucun
reset3600. Anciennes portées rouges restent historiques. Revue/GO EXACT
requis avant runtime. Max32GiB, CPUj8/native29/busunique inchangés.
