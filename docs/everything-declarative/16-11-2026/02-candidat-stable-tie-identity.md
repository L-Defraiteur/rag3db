# Ordre stable des occurrences : candidat après reprise

## État et contrat

Audit et préparation autorisés par C7 après la passation16 ; aucune passe lourde pendant la fenêtre de bascule G/Arbre. Source de départ publiée `1834bf1114bf6894f6554754e11c41b2614db4fa`, branche historique `stable-tie-identity` conservée intacte. Candidat produit **`9fcc98ad3f347d821f3f1b99c936a73a9e080d41`**, branche neuve `cx-ed-stable-tie-identity-r1`, basé sur master **`927941a67a5805450d49f1e04b39fdd542a6059b`**. WT portable `/home/lucied/git_workspaces/rag3db-cx-ed-stable-tie-identity-r1` ; aucun WT ou target de ce candidat créé à luciepc à cette étape.

Ticket existant : [les ex æquo de la recherche n'ont pas d'ordre stable](../../tickets/closed/2026-11-16-les-ex-aequo-de-la-recherche-n-ont-pas-d-ordre-stable.md). Sa première correction de coupe est sur master. Le complément en branche ne remplace pas cette preuve et ne rouvre pas artificiellement l'ensemble de l'historique.

Master sait demander les ex æquo de la coupe à Lucivy, dans la borne existante4096. Mais son départage par uuid d'occurrence varie quand une réécriture fait renaître la ligne sous un autre `_depuis`. Ce complément trie par score, puis `file_path`, `start_line`, `name`, et enfin uuid comme départage résiduel ; sans chemin, il garde l'identité uuid du contenu ou de la clé. Il applique la même primitive `stable_order` à BM25, aux remontées vectorielles, aux projections d'occurrences et à la fusion. Il ne modifie ni score, visibilité, identité persistée, schéma ou mécanisme de coupe Lucivy.

## Diff et réservation

Trois fichiers Rust seulement :

- `extension/rag3weaver/src/search.rs` : primitive commune de comparaison et ses sites de tri ;
- `extension/rag3weaver/src/dataflow/generic_search_nodes.rs` : tri de la fusion par cette primitive ;
- `extension/rag3weaver/src/backend.rs` : témoin uniquement, aucune modification de chemin backend de production.

`docs/nouveautes.md` reprend la ligne du lot historique. Les présentes docs ajoutent sa trace de reprise. C7 réserve ces trois chemins ; Recherche prépare ailleurs et atterrit après ED. Embarquement réserve ses cinq chemins rate-key, sans chevauchement avec ce candidat ; pour Vertex ensuite, ne pas confondre `model_source.rs` et son sous-module `model_source/activation.rs`.

Contrôles RO effectués : quatre `range-diff '='` entre les historiques ea4704/bb38c2/a4d1c5/1834 et f969c1/b606d5/5d631f/9fcc ; aucun conflit ; diff produit candidat→1834 vide ; `git diff --check` vert. La fourche réelle historique est9ed0c640, et non le repère fe5 plus ancien cité dans la passation. Aucun delta sous `extension/rag3weaver` de cette fourche à master927 ; le pointeur Lucivy4.3.9 était déjà présent. Aucun nouveau test exécuté pour ces contrôles.

## Preuves historiques, non transférées en verdict candidat

Témoin `backend::tests::ties_keep_the_same_order_from_a_rewritten_base_to_a_fresh_one` : six fonctions de même score, baseX écrite/publiée/réécrite et baseY écrite une seule fois ; même ordre final exigé. Le témoin ne déduit pas un ordre stable de l'uuid `_depuis`. Rouge exécuté rapporté sur ea4704bf6, vert sur a4d1c59db ; la passation16 est la source de ces résultats.

Passe historique1834 rapportée : F8 8/8, lib2595/0/72, NAV45, e2e_code36, e2e_lectures27. Les ignorés restent non qualifiés. Ces comptes ne sont pas une nouvelle passe ni un résumé candidat ; les journaux historiques sont à conserver. Leur localisation précise n'a pas été retrouvée lors de cette préparation, donc aucun SHA de journal ou copie nouvelle n'est inventé. La garde décide sur les preuves réelles disponibles ; aucun résumé composé à la main.

## Recette après FINI G seulement

1. Attendre l'annonce explicite **FINI G**, le lot qui épingle la lib neuve et la copie courante vérifiée de `passe_epinglee.sh`/`push_garde.sh`. Lib attendue selon plan G : `rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a` ; son identité effective sera lue dans l'annonce et la première ligne du résumé. Retirer `RAG3WEAVER_MOTEUR_ANCIEN` seulement conformément à cette annonce. Ne pas installer soi-même les scripts ni les services pendant la fenêtre.
2. Fetch frais et audit du delta master. S'il faut porter à nouveau le lot, utiliser encore une branche `cx-` NEUVE, jamais de force. Relever sources, lib, gitlink et target exacts avant toute qualification. WT de qualification proposé à luciepc `~/git_workspaces/rag3db-cx-ed-stable-tie-identity-r1`, target session proposé `~/.cache/rag3weaver-build/target-cx-ed-stable-tie-identity`, absence/provenance à vérifier avant création ou adoption. Aucun target créé ici.
3. Sous vrai `poste lourd`, `POSTE_MEM_MAX=32G`, au plus j8 et timeout global fixé avec C7 : témoin exact sur la source candidate avec `scripts/cargo_magasin.sh`, puis passe épinglée actuelle. Proposition conservatrice : `SUITE_DU_TEMOIN=e2e_code`, `SUITES=e2e_lectures` ; F8, lib entière et NAV de l'outil restent obligatoires. Les trois chemins Rust de ce candidat ne déclenchent pas à eux seuls `e2e_code_sync` dans `required_suites.txt` ; si le delta réel change ces chemins requis, la passe l'ajoutera. Ne pas déduire une dispense de la seule identité produit avec1834 : la lib native change.
4. Lire les comptes/noms du témoin et les résumés réellement produits. Premier rouge : arrêt, preuve conservée, diagnostic puis décision distincte ; pas de retry automatique ni skip. Après succès, annoncer le candidat exact et son journal à C7. Push master uniquement à son **À TOI** courant via garde/`poste push`, puis contrôles usuels. Aucun ancien délai ou tour ne reprend.

Le prochain lot est le [jeton Vertex](../../banc/tickets/2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md), séparé : faux HTTP et source de jeton factice pour le rouge, aucun appel cloud requis. Ses fichiers proposés `model_source.rs`, `openai_llm.rs`, `gcp_auth.rs` n'entrent pas dans le présent candidat.
