# La règle provisoire « pas de température pour Gemini 3 » est un test de nom dans le code

- **État** : ouvert, le 23 novembre 2026 ; part avec le lot `model-temperature`
- **Famille** : produit
- **Gravité** : basse : une entorse temporaire à la règle de Lucie (rien en dur dans le code, tout par la déclaration du modèle)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/openai_llm.rs`, `request_body`)
- **Pour** : archi
- **Signalé par** : archi, avec l'accord de c7, le 23 novembre 2026

## Ce qui est en place

Le guide de Google (« Get started with Gemini 3 ») demande de garder la température par défaut (1,0) : en dessous, le
modèle peut boucler. Pour protéger la démonstration de Lucie dès le lot `step-done-needs-text`, `request_body`
n'envoie ni `temperature` ni `top_p` quand le client est Google et que le nom du modèle contient `gemini-3`. C'est un
test de nom, donc une liste en dur.

## La décision de Lucie (23 novembre 2026, vers 17 h 1x)

Gemini 3 : aucune température. Les autres modèles : environ 0,666 par défaut, ramené dans la plage du modèle (rien
si le modèle n'en accepte pas). Le tout déclaré par modèle (`temperature`, `temperature_range`), le défaut 0,666 dans
la configuration de l'équipier ; « pas de température pour Gemini 3 » porté par la déclaration de ce modèle, jamais
par le code.

## À faire

Le lot `model-temperature` pose ces déclarations (dont celle du modèle par défaut de l'hôte, qui n'en a pas
aujourd'hui), avec ses témoins (rien pour Gemini 3, 0,666 pour un autre, une valeur hors plage ramenée), et retire le
test de nom.
