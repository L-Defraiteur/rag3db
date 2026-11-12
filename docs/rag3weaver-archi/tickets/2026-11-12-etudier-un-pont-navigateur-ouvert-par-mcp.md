# Étudier un pont navigateur ouvert par MCP

- **État** : ouvert ; futur fork souhaité par Lucie, trois candidats repérés, étude comparative différée et aucun prototype lancé.
- **Famille** : produit
- **Gravité** : planification ; permettre aux agents d'utiliser le navigateur depuis l'atelier.
- **Atteignable en service** : non pour ce raccordement ; aucune capacité livrée par cette note.
- **Touche rag3weaver** : oui, intégration MCP et outils de l'atelier, accords et observation des actions.
- **Pour** : archi et ED, calendrier suivi par orchestration.
- **Signalé par** : Lucie le 12 novembre : chercher plus tard une extension de type assistant pour Chrome, open source et généraliste aux agents par MCP.

## Demande

[Vision et critères de comparaison](../../orchestration/visions/2026-11-12-un-pont-navigateur-generaliste-par-mcp.md).
Rechercher des projets existants avant de décider s'il faut une extension ou un
adaptateur. Distinguer l'accès aux onglets d'une session humaine et un navigateur
piloté séparément. Vérifier licence, outils MCP réels, maintenance, dépendances,
profils, données transmises et intégration aux contrats d'accord et d'annulation.

## Fin attendue

Comparaison sourcée à une date et une version, écarts avec le besoin et
recommandation motivée du socle à forker et des adaptations nécessaires.
Ne pas fermer ce ticket pour la seule conservation de l'idée. L'installation,
le raccordement à un profil humain et une éventuelle intégration restent des
étapes distinctes à planifier après l'étude.

## Candidats proposés par Lucie

`EmilLindfors/rust-browser-mcp`, `BB-fat/browser-use-rs` et la crate
`browser_oxide_mcp` de `yfedoseev/browser_oxide`. Liens figés, premiers éléments
et écart de description du troisième dans la vision : son moteur actuel est
propre, pas chromiumoxide. Ne pas confondre identité simulée Firefox et moteur
Firefox réellement piloté. Direction retenue : forker l'un des projets après
comparaison ; aucun fork distant créé par cette mise à jour documentaire.

Critère ajouté par Lucie : exécution JavaScript à la demande dans la page.
`rust-browser-mcp` expose bien `execute_script` et le route vers WebDriver ;
schéma, gestionnaire et limite de qualification de l'asynchrone documentés dans
la vision. Vérifier le comportement réel, les délais et le retour des valeurs
avant d'intégrer cette capacité.
