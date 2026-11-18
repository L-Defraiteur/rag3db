# Embarquement : la cause d'arrêt d'un service posé à la main (conception)

Entrée 7 de `docs/embarquement/16-11-2026/01-passation-pour-codex.md`, ticket
`embarquement/tickets/2026-11-15-la-cause-d-arret-d-un-service-pose-a-la-main-n-est-pas-dite-par-l-hote.md`.
Conception soumise à c7 le 18 novembre au soir ; **rien n'est codé avant son mot.**

## Le manque

Un fournisseur lancé par une **activation** écrit dans `<activation.journal>/rag3weaver-embeddings.log`. À sa mort,
l'hôte y lit la dernière ligne « ✗ arrêt : <cause> (code N) » (`cause_d_arret`, `src/model_source/activation.rs:1048`)
et dit « service d'embarquement arrêté : <cause> ».

Un service **posé à la main** (les unités `rag3weaver-embeddings@` de luciepc, 7878 et 7879) écrit dans le fichier que
nomme son drop-in `journal.conf`, que la déclaration ignore. L'hôte ne peut dire que « service déclaré mais pas joint
(127.0.0.1:7879) — démarrez-le » (`raison_sans_service`, `src/backend.rs:2569`), même quand le journal sait pourquoi.

## La conception

1. **Un champ optionnel `log`** dans une déclaration `models.<capacité>` (`ModelSource`, `src/model_source.rs`) : le
   chemin **absolu** du journal du service posé. Le nom est en anglais, comme tout code neuf. Un chemin relatif se
   refuse au chargement, nommé. Le champ est sans effet pour une activation, qui a déjà son `journal` ; les deux
   ensemble se refusent au chargement, pour qu'il n'y ait pas deux vérités.
2. **Une seule lecture** : `cause_d_arret` devient `stop_cause(source)` (alias français gardé) et lit, dans l'ordre,
   le journal de l'activation locale, puis `source.log`. L'analyse ne change pas : la dernière ligne non vide, si elle
   commence par « ✗ arrêt : », sans le « (code N) ».
3. **Là où elle se dit** :
   - `raison_sans_service` (`backend.rs`) : quand le service déclaré n'est pas joint **et** que `stop_cause` rend une
     cause, la raison devient « service d'embarquement arrêté : <cause> (127.0.0.1:7879) — il est repris dès qu'il
     répond, et l'index suivant rattrape la dette ». Sans cause, la phrase d'aujourd'hui (« pas joint — démarrez-le »).
   - Cette raison va déjà au reçu, à l'état d'index et à la retente (`RetenteEmbarqueur`) : rien d'autre à brancher.
4. **Une cause périmée ne trompe pas** : un service relancé écrit d'autres lignes après son « ✗ arrêt », donc la
   dernière ligne n'est plus une cause. Et la raison ne se lit que si le service ne répond pas.
5. **Ce qui ne se touche pas** : les unités `rag3weaver-embeddings@` et leurs drop-ins (G ou Lucie) ; le format de la
   ligne d'arrêt du service ; les manifestes de Lucie. Le chemin des journaux de luciepc
   (`~/.cache/rag3weaver-unites-embarquement/journal-<instance>.log`) se dit dans le README, comme exemple ; l'ajouter
   à un manifeste réel est la décision de sa propriétaire.

## Le témoin, rouge d'abord

`a_posed_service_stop_cause_is_said_from_its_log` (`src/backend.rs`, tests ; `daemon`, `unix`) :
- la fixture `fournisseur.py` est lancée à part (hors activation, comme `poser_un_service` dans les tests
  d'`activation.rs`), avec `--arret-nomme` et sa sortie d'erreur dans un fichier ; son premier `/embed` écrit
  « ✗ arrêt : le calcul a paniqué (…) (code 70) » et la fait sortir ;
- la déclaration `models.embed` donne son adresse et `"log": <ce fichier>` ;
- `embedder_or_absent()` rend la raison « service d'embarquement arrêté : le calcul a paniqué … » ;
- sans `log`, la raison reste « pas joint ».

Rouge d'abord : le champ `log` n'existe pas encore, et le stub l'accepte sans le lire.

## Taille

Une demi-journée : 1 h pour le code, 1 h pour le témoin et la passe.
