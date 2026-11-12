# La CI cherche le bâti vectoriel dans un dossier absent

- **État** : ouvert ; échec du job relu, sélection du dossier de bâti à diagnostiquer.
- **Famille** : produit
- **Gravité** : CI rag3weaver bloquée avant l'exécution des tests unitaires.
- **Atteignable en service** : non, workflow de validation.
- **Touche rag3weaver** : oui, .github/workflows/rag3weaver-workflow.yml.
- **Pour** : optimiseur, coordination orchestration pour l'intégration.
- **Signalé par** : orchestration, vérification de la CI après l'intégration du lot A.
- **Ouvert le** : 12 novembre 2026

## Preuve relue, sans rerun

[Run rag3weaver43287259623](https://github.com/L-Defraiteur/rag3db/actions/runs/43287259623),
source6dd116e8f70fd328acf3b45a89401e06f71365cb, job131426574887,
étape8 « L'extension vector, du même bâti », échec le12novembre19:46:35UTC.
Le connecteur fournit les métadonnées du job ; le journal a été lu via gh API,
conservé avec une copie sans séquences ANSI dans
`~/.cache/orchestration-notes/ci-12nov-2103/`.

La commande cherche
`extension/rag3weaver/target/debug/build/rag3db-*/out/build` par ls/mtime.
Le glob ne trouve rien ; CMake reçoit un dossier vide après -B :

```text
ls: cannot access 'extension/rag3weaver/target/debug/build/rag3db-*/out/build': No such file or directory
CMake Error: No build directory specified for -B
```

Le même journal montre la compilation de rag3db0.11.1 depuis tools/rust_api,
puis un exécutable de tests sous
`target/debug/build/rag3weaver/024bb70f754e3a61/out/`.
Cela est un indice de disposition différente, pas la preuve du chemin CMake
effectif. Ne pas remplacer le glob par un autre chemin deviné ni choisir un
cache ancien seulement par sa date. Aucun défaut du code A ou des index réels
n'est établi par cet échec antérieur à leur intégration.

## Travail borné et fin attendue

Relire le workflow et le producteur Cargo/CMake réel ; identifier le dossier
du **même** moteur que celui lié aux tests. Composer les informations existantes
(notamment artefacts et build-script-executed Cargo si pertinents), avec refus
nommé quand la provenance est absente, périmée ou ambiguë. Publier un diff limité
au workflow et à l'éventuel helper commun nécessaire ; préserver source moteur,
configuration, ABI et extension produite, sans nouvelle lib posée sur les postes.

Éprouver la sélection et les refus par fixtures CPU représentatives, à partir
des faits du producteur observé ; ne pas écrire de témoin qui valide seulement
le nouveau glob choisi. Aucun rebâti C++, Cargo complet, service, modèle ou GPU
nécessaire au diagnostic initial. Premier rouge inattendu : arrêt et diagnostic,
pas de rejeu inchangé. Tout besoin d'une validation lourde se coordonne après
le diff ; la CI automatique future jugera le commit intégré. Aucun rerun des
anciens jobs ni publication de paquet ou modification d'identifiants.

Fin : dossier sélectionné avec provenance explicite du même bâti, refus des
cas absents/ambigus/périmés, diff et preuves relus puis intégrés. Le ticket se
clôture avec ce verdict, sans présenter une validation CPU comme une CI verte.
