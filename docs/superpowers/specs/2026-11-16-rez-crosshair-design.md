# REZ — Crosshair (phases 2 et 3)

Complète [2026-11-16-rez-design.md](2026-11-16-rez-design.md), sections 2 et 3. Les décisions ci-dessous priment sur le design initial quand elles diffèrent.

## Demande

- Un système de crosshair **lié aux jeux** : chaque jeu de la liste choisit son crosshair, ou « Aucun ».
- **5 crosshairs proposés d'office** à l'installation.
- Les imports dans la même livraison : code Valorant, code CS2, image PNG.

## Décisions

### Bibliothèque

- `Config.crosshairs: Vec<Crosshair>`.
- Au premier lancement (`settings.crosshairsSeeded == false`), REZ ajoute 5 presets, puis passe le drapeau à `true` : si l'utilisateur supprime tout, rien ne revient.
- Les 5 presets reprennent les styles les plus utilisés par les pros (décision de l'utilisateur, 2026-11-19) : petits, statiques, sans contour, surtout cyan ou vert. Unités en px à 1080p. Aucun nom de joueur n'est utilisé.

| # | Nom | Couleur | Forme |
|---|---|---|---|
| 1 | Cyan pro | cyan `#00FFFF` | longueur 4, épaisseur 2, écart 2, sans contour (classique Valorant pro) |
| 2 | Plus sans écart | blanc `#FFFFFF` | longueur 4, épaisseur 2, écart 0, sans contour |
| 3 | Vert statique | vert CS `#32FA32` | longueur 4, épaisseur 2, écart 2, sans contour (consensus CS2 : taille 2, écart −3, épaisseur 1) |
| 4 | Point | blanc `#FFFFFF` | point de 4 px, contour noir 1 px |
| 5 | Fin jaune | jaune `#FAFA32` | longueur 5, épaisseur 1, écart 2, sans contour (CS2, épaisseur 0) |

- Opérations : créer, dupliquer, renommer, supprimer, éditer.
- Supprimer un crosshair utilisé par un jeu remet ce jeu sur « Aucun ».

### Modèle (`src-tauri/src/crosshair/model.rs`)

Un seul modèle sert pour le rendu, l'éditeur et les imports :

- `Crosshair` :
  - `id`, `name` ;
  - `color` : `#RRGGBB` ;
  - `inner` et `outer` de type `Lines` ;
  - `tStyle` ;
  - `outline { enabled, thickness, opacity }` ;
  - `dot { enabled, size, opacity }` ;
  - `circle { enabled, radius, thickness, opacity }` ;
  - `image: Option<String>` (fichier dans `%APPDATA%\REZ\crosshairs\`) et `imageScale` ;
  - `offsetX`, `offsetY`.
- `Lines` : `{ enabled, length, verticalLength: Option, thickness, gap, opacity }`.
- `outer` existe pour reproduire les crosshairs Valorant. Les presets ne l'utilisent pas.

### Lien au jeu

- `GameEntry.crosshairId: Option<String>`. Défaut : `None` (« Aucun »), pour éviter deux viseurs superposés.

### Affichage

- **Quand** : seulement si le process au premier plan est un jeu configuré qui a un crosshair, que l'affichage n'est pas coupé par le raccourci, et que FACEIT ne tourne pas.
- **Fenêtre** :
  - Win32 layered, transparente et non cliquable (`WS_EX_LAYERED | TRANSPARENT | TOPMOST | TOOLWINDOW | NOACTIVATE`) ;
  - rendue par `tiny-skia`, puis `UpdateLayeredWindow` ;
  - aucune injection dans le jeu.
- **Position et taille** :
  - centre de la zone client de la fenêtre au premier plan, plus le décalage X/Y ;
  - REZ revérifie la position 4 fois par seconde tant que le crosshair est visible, car le jeu se redimensionne après le changement de résolution.
- **Échelle** : `hauteur du mode courant / 1080`.
- **Anti-déformation** (activée par défaut, `settings.crosshairUndistort`) : facteur X = `(mode_w / native_w) / (mode_h / native_h)`.
- **Raccourci Ctrl+Alt+Shift+X** : cache ou réaffiche le crosshair pour la session.
- **FACEIT** : process `faceit.exe`, `FACEIT.exe` ou `faceitservice.exe` en cours d'exécution → crosshair caché (`settings.hideCrosshairWithFaceit`, activé par défaut).
- **Limite connue** : invisible en plein écran exclusif. C'est la décision « étirement prioritaire » du design initial.

### Imports

- **Code Valorant** (`0;P;…`) :
  - les valeurs par défaut de Valorant sont complétées par les clés du code ;
  - les clés liées au mouvement ou au tir sont ignorées, avec une note.
- **Code CS2** (`CSGO-…`) :
  - décodage en octets, puis conversion vers le modèle (une seule série de branches) ;
  - les champs dynamiques (split, follow recoil) sont ignorés, avec une note.
- **Image PNG** : copiée dans `%APPDATA%\REZ\crosshairs\<id>.png`, avec taille (`imageScale`) et opacité réglables.
- **Code REZ** : `REZ1-<base64url(json du Crosshair sans id)>`, en export et en import.
- **Champ unique « Coller un code »** : il détecte le format (`REZ1-`, `CSGO-`, ou commence par `0;`).

### Interface

- **Onglet « Crosshair »** :
  - liste des crosshairs avec vignettes ;
  - éditeur : grand aperçu sur fond de jeu sombre, curseurs et interrupteurs ;
  - boutons : Nouveau, Dupliquer, Supprimer, Coller un code, Importer une image, Copier le code REZ.
- **Carte de jeu** : un sélecteur « Crosshair » avec vignettes, dont « Aucun ».
- **Réglages** : anti-déformation, cacher si FACEIT tourne, rappel du raccourci.
- **Aperçus** : commande Rust `preview_crosshair(crosshair) -> data URL PNG`, avec le même moteur que l'overlay.
