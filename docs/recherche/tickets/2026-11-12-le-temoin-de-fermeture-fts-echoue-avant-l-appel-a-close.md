# Le témoin de fermeture FTS échoue avant l’appel à close

- **État** : ouvert, échec CI exact08086ba36 inspecté ; diagnostic et correction du montage à confirmer.
- **Famille** : produit
- **Gravité** : blocage de la validation CI ; le témoin peut échouer avant de juger le contrat de fermeture.
- **Atteignable en service** : non pour ce panic de préparation du test ; aucune régression produit de close établie.
- **Touche rag3weaver** : oui, src/fts_handle.rs::repro_drop et validation CI.
- **Pour** : recherche ; coordination lucivy si un défaut amont distinct est réellement reproduit.
- **Signalé par** : orchestration, contrôle périodique de la CI le12novembre2026.

## Preuve exacte

Commit08086ba3600a73f4828e541b093cb8751a1984fb ;
[run rag3weaver43187074046](https://github.com/L-Defraiteur/rag3db/actions/runs/43187074046),
[job131076793291](https://github.com/L-Defraiteur/rag3db/actions/runs/43187074046/job/131076793291).
Étape cargo test --lib sous rag3db-native,code,daemon,openai-llm.
Résultat2089 réussis,1 échec,64 ignorés,2 filtrés ;117,96s.

Le seul échec est
`fts_handle::repro_drop::une_fermeture_fts_en_erreur_rend_le_handle_inerte_et_le_processus_sort`.
L’enfant panique à fts_handle.rs652 sur remove_dir_all(index).unwrap :
`Os { code: 39, kind: DirectoryNotEmpty, message: "Directory not empty" }`.
Le log signale aussi un fold de dictionnaire en arrière-plan échoué à ouvrir
`index/shard_0/dict-1.1.sfx.fst.tmp`. Le parent rapporte ensuite l’échec de l’enfant.
Le close que ce témoin doit mettre en défaut est APRES la suppression qui a paniqué.
Aucun résultat sur son inactivité après close n’est établi par ce run rouge.

## Diagnostic actuel et limites

La suppression récursive est déclenchée alors que le handle et ses acteurs sont vivants.
Une création de fichier concurrente est une cause plausible d’ENOTEMPTY, à reproduire ;
ne pas remplacer la preuve par une supposition sur le runner. Le fichier fts_handle.rs
est strictement inchangé entre f96acc6cf (mise à jour4.3.3) et08086ba36 : aucune causalité
avec le partage de contenu1a n’est démontrée. La passe locale831d reste verte mais ne
remplace pas cette CI. L’ancien ticket du vrai close en erreur reste clos ; ce ticket
ne rouvre pas ce correctif amont sans reproduction distincte.

## Travail attendu

Reprendre le montage sur un worktree privé, conserver le log rouge et reproduire le
point de préparation défaillant. Rendre l’injection d’erreur de persistance déterministe
par une primitive publique ou une fixture maîtrisée, sans dépendre d’une suppression
concurrente non synchronisée, sans sommeil/réessai aveugle et sans modifier la bibliothèque
si aucun défaut produit n’est établi. Le même contrat enfant reste vérifié : close
échoue réellement, le handle refuse ensuite l’indexation et Drop/processus se terminent.
Les tests n’ouvrent que leurs ressources privées ; aucun service, modèle ni index réel.

## Critères de fin

Témoin fiable qui atteint l’erreur close voulue, garde les assertions d’inactivité et de
sortie bornée, avec rouge/vert attribués aux bonnes sources ; contrôles adaptés au jeu CI
puis passe requise, revue et intégration. Ni ignore/exemption, ni suppression d’assertion,
ni rerun de CI pour tenter la chance. La CI future se lit sur son SHA exact ou la première
exécution terminée plus récente selon la procédure.
