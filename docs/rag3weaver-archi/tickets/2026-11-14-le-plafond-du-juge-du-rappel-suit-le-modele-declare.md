# Le plafond du juge du rappel suit le modèle déclaré

- **État** : ouvert (14 novembre 2026), pour archi, à faire au lot où le juge différé s'allume
- **Famille** : produit
- **Gravité** : réponse coupée à venir — le plafond de sortie du juge est une constante
  (`PLAFOND_DU_JUGE`, 2 048) ; un modèle qui réfléchit plus que Gemini 3.8 Flash rejouerait la coupe
  du 8 novembre
- **Atteignable en service** : non aujourd'hui, le juge n'est allumé chez personne (aucun verdict
  dans `hooks.log`)
- **Touche rag3weaver** : oui (`src/claude_code_hook.rs`, `src/model_source.rs`)
- **Pour** : archi

## Ce qui est décidé (c7, 14 novembre 2026)

Forme (a) : un champ `max_output_tokens` sur la source de modèle (`models.llm`), défaut 2 048, lu
par `Decide` et par les juges qui ont un manifeste ; le juge du rappel, dont le modèle vient du
régime (`regime::modele_agentique_nomme`), le prend de la même déclaration quand il en a une, sinon
le défaut. Le précédent fermé : `docs/tickets/closed/2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md`
(`52021b977`).
