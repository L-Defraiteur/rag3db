# Étudier un pont navigateur ouvert par MCP

- **État** : ouvert ; Chrome visible avec extension et passage de main humain, cœur composable en Rust ; transcription ciblée du pont à étudier avant un portage complet, aucun prototype lancé.
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

## Précision : extension et intervention humaine

Lucie ne cherche pas seulement un navigateur automatisé séparé : elle doit
pouvoir intervenir pour une connexion, un double facteur ou une page de clé
API, puis laisser l'agent reprendre dans le même onglet/session. Headless reste
optionnel. Le choix du socle doit privilégier ce parcours avant la langue Rust.

Playwright MCP/Playwright Extension, BrowserMCP/mcp et jackwener/browsermcp
ajoutés avec sources figées dans la vision ; les deux BrowserMCP sont distincts.
Comparer les sources complètes de l'extension et du serveur, le rattachement à
un onglet existant, l'exécution JavaScript, la pause/reprise après observation,
l'annulation et la topologie agent sur luciepc/navigateur sur portable.

Pour qualifier le futur prototype : Lucie peut rejoindre une page de connexion,
agir pendant que l'agent attend, puis la même session reprend après observation
de l'état réel. Vérifier aussi intervention annulée, onglet fermé ou navigation
humaine inattendue. Aucun test sur compte réel ni installation engagé ici.

## Précision : composabilité Rust et transcription ciblée

Lucie demande de pouvoir réutiliser les capacités dans notre code Rust, au-delà
d'un serveur MCP externe. Elle propose explicitement de transcrire seulement
la partie qui parle à l'extension si son volume le permet, en gardant l'extension
existante. Cette piste est prioritaire dans l'étude du futur fork.

Identifier messages/transport/appairage et frontière extension/hôte ; mesurer
code propre et dépendances avant d'estimer le portage. Proposer un cœur Rust
appelable comme bibliothèque par rag3weaver, et une façade MCP sur les mêmes
capacités pour les agents externes. Pas de logique métier dupliquée dans la
façade ni d'orchestrateur d'agents importé par défaut.

La comparaison attendue doit dire si ce petit pont suffit pour observation,
actions, JavaScript, sessions et passage de main, ou quelles fonctions de l'hôte
amont imposeraient un portage plus large. Contrat et schéma dans la vision ;
aucune implémentation ni installation déclenchée par cette précision.
