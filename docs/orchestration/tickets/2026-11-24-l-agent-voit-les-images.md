# L'agent voit les images

- **État** : ouvert (24 novembre 2026, orchestration).
- **Gravité** : fonction neuve — l'agent sans modèle de vision ne lit pas une image collée.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (le terminal, l'équipier).
- **Famille** : produit
- **Pour** : rag3weaver archi, avec tty.
- **Signalé par** : Lucie.

Ouvert le 24 novembre 2026, 13 h 10. Lucie, le même jour, en faisant la recette de `tisserande` : « chose prio : il n'a
pas de modèle de vision, il faut pouvoir lui copier-coller des images, et il doit aussi pouvoir lire des images depuis un
fichier ; un modèle de vision la décrit pour lui, avec un prompt de description par défaut qu'il peut remplacer en
option ».

Relevé (c7) : le client des modèles (`src/openai_llm.rs`, `src/llm.rs`) n'envoie aujourd'hui que du texte ; aucun outil
ne lit une image ; `read_file` sur une image ne rend rien d'utile. Le terminal ne colle que du texte.

## Ce qui est attendu

1. **Coller une image dans le terminal** (et dans le chat de l'IDE) : Ctrl-V sur une image du presse-papier
   (`wl-paste --type image/png` sous Wayland, présent à la tablette) l'enregistre dans le dossier de la tâche
   (`tmp/` ou `images/`) et joint son chemin à la demande : « [image collée : <chemin>] ». Une commande `/image <chemin>`
   fait de même depuis un fichier.
2. **Un outil `read_image`** : `path` (une image du dossier, ou collée), `prompt` (facultatif). Il rend la description
   de l'image par un modèle de vision.
   - **Le prompt de description par défaut** est un gabarit livré (`templates/teammate/prompts/describe_image.md`) :
     décrire ce qui est visible, le texte lu tel quel, la mise en page, ce qui semble cassé ou vide (une interface :
     ses contrôles, leur état, les zones blanches, les icônes d'image manquante). L'agent le remplace par `prompt`
     quand il cherche quelque chose de précis (« la scène 3D s'affiche-t-elle à droite ? »).
   - **Le modèle de vision** est déclaré (un nom par variable, jamais en dur) ; par défaut, le modèle de l'équipier s'il
     est multimodal (les Gemini le sont), sinon un modèle déclaré pour la vision.
3. **Mieux qu'une description quand le modèle de l'équipier voit** : déclaré par modèle (`vision: true`), l'image part
   directement dans le message au modèle, en partie image, et la description n'est qu'un repli. À mesurer : une image
   jointe coûte des jetons à chaque appel du tour ; la description, une fois.
   **Les deux chemins existent, choisis par la capacité déclarée** (Lucie, 24 novembre 2026 : « c'est un cas spécial,
   d'autres modèles ne l'ont pas, donc savoir avoir les deux sur capacité déclarée ») : chaque modèle du registre déclare
   `vision: true | false`. Vrai : l'image part en partie image. Faux, ou non déclaré : elle est décrite par le modèle de
   vision déclaré, et l'équipier reçoit la description. Jamais de choix par le nom du fournisseur ou du modèle dans le
   code. Le même `read_image` sert les deux, et dit lequel il a pris.
4. Une image collée ou lue est un fait du tour : son chemin et sa description passent d'un pas à l'autre.

## Le témoin

Par un vrai hôte et un faux modèle : une demande qui joint une image collée fait appeler le modèle de vision avec le
prompt par défaut, et l'agent reçoit la description ; `read_image` avec un `prompt` le remplace. Puis, vu à l'écran en
saisie humaine : coller une capture dans le terminal, et l'agent la décrit.

## Le cas qui l'a fait naître

La capture de Lucie du 24 novembre, 13 h : l'application de `tisserande` lancée, la barre d'outils affichée,
« DAG Géométrie — 576 faces (625 sommets) », une icône d'image manquante en haut à gauche, et tout le reste blanc : ni
la scène 3D, ni le panneau du graphe à demi-écran demandé. L'agent ne pouvait pas la voir.
