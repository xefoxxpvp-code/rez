# Plan — REZ : résolution par jeu + crosshair

## Contexte

Beaucoup de joueurs compétitifs jouent en 4:3 étiré (1440x1080, 1280x960…). Aujourd'hui il y a deux options, et aucune ne marche bien :
- **Réglage dans le jeu** : souvent impossible, Valorant et Fortnite ajoutent des bandes noires.
- **Changer la résolution Windows** : Chrome, Discord et le bureau sont étirés aussi.

Objectif : une app Windows **gratuite et publique** qui :
1. applique une résolution choisie **seulement quand le jeu est au premier plan** et revient en natif dès qu'on le quitte ;
2. affiche un **crosshair perso identique dans tous les jeux**, à importer depuis un code de jeu, une image ou un éditeur.

Jeux v1 : Valorant, Counter-Strike 2, Fortnite, Apex Legends, Rust, ROB RUN (Steam app 4262460).

**Objectifs**
- Le bureau n'est jamais étiré hors du jeu.
- On ne reste jamais bloqué dans une mauvaise résolution, même si l'app plante.
- Rien n'est injecté dans les jeux, pour rester sûr face aux anti-cheat.
- L'app est légère en fond.

**Hors scope v1** : macOS/Linux, boost FPS, macros, enregistrement, comptes utilisateurs, version payante. Le mode de secours « copie étirée de la fenêtre » (capture ou miniature DWM) est aussi hors v1 : écarté par l'utilisateur, il pourra être ajouté plus tard si un jeu refuse le changement de résolution.

**À ne jamais faire**
- Injection DLL, lecture mémoire ou hook DirectX (risque de ban).
- Écrire la résolution de façon permanente dans le registre.
- Une app lourde en RAM.

## Approches (choix fait)

| Option | Verdict |
|---|---|
| **Tauri 2 (Rust + React/TS)** | **Retenu.** Installateur < 10 Mo. Peu de RAM : la WebView est fermée quand l'app est réduite près de l'horloge. Mises à jour intégrées. Rust parle directement à Windows via le crate `windows`. |
| C# .NET 8 + WPF | Viable, gardé en secours. Exe self-contained d'environ 70 Mo, interface moins moderne, mises à jour via un outil tiers (Velopack). |
| Electron | Rejeté : plus de 150 Mo et 200 Mo de RAM pendant qu'on joue. |
| Capture de fenêtre étirée (style Lossless Scaling) | Rejeté : ajoute de la latence, inacceptable en compétitif. |

## Mécanismes clés

### 1. Résolution par jeu (`src-tauri/src/display.rs`, `watcher.rs`)
- **Détection du premier plan** : `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, puis `GetWindowThreadProcessId` et `QueryFullProcessImageNameW`. On compare le nom de l'exe avec les jeux configurés.
- **Machine d'état** `Idle ↔ GameActive(game_id)` :
  - entrée immédiate ;
  - sortie après un délai de 250 ms, pour ignorer les popups brèves ;
  - chaque jeu a une option « rester étiré pendant alt-tab ». Dans ce cas on revient en natif seulement à la fin du process, surveillée par `OpenProcess` + attente.
- **Changement de mode** :
  - On récupère l'écran du jeu avec `MonitorFromWindow`, puis `GetMonitorInfoW` donne son `szDevice`.
  - On applique le mode avec `ChangeDisplaySettingsExW(device, &devmode, CDS_FULLSCREEN)`. Ce mode est temporaire et jamais écrit dans le registre.
  - **On garde la fréquence native** (`dmDisplayFrequency`, 144/240 Hz).
- **Retour au natif** : `ChangeDisplaySettingsExW(device, NULL, 0)`, qui relit le mode du registre. Ce retour se fait dans tous les cas :
  - sortie du jeu ;
  - fermeture de l'app ;
  - panic hook Rust ;
  - `WM_ENDSESSION` ;
  - raccourci d'urgence.
- **Résolutions proposées** : uniquement celles que liste `EnumDisplaySettingsExW` pour cet écran. Si 1440x1080 manque, l'app explique comment la créer (panneau NVIDIA/AMD, ou CRU).
- **Tests** : `display.rs` est derrière un trait `DisplayApi`, avec une implémentation mock pour tester la machine d'état.
- **Effets de bord sur le bureau.** Réduire la résolution déplace les icônes et les fenêtres de l'écran concerné. Avant de basculer, l'app enregistre :
  - les positions des icônes, avec `IShellWindows::FindWindowSW(SWC_DESKTOP)` → `IFolderView::GetItemPosition` ;
  - les rectangles des fenêtres de cet écran.
  Au retour en natif, elle remet tout en place avec `SelectAndPositionItems` et `SetWindowPos`.
- **Retours en natif provoqués par Windows.** UAC, Ctrl+Alt+Suppr ou le verrouillage peuvent remettre le mode natif. On écoute `WM_DISPLAYCHANGE` : si le jeu est toujours au premier plan, on réapplique le mode.
- **Spike au début de la phase 1.** Vérifier si Windows restaure le mode `CDS_FULLSCREEN` quand le process qui l'a posé meurt (kill brutal). Sinon, ajouter un mini process « gardien » qui restaure le natif si l'app principale disparaît.
- **Jeux qui ne se redimensionnent pas en cours de partie.** Option par jeu « appliquer dès le lancement » : on détecte le process par polling Toolhelp toutes les secondes et on bascule avant que sa fenêtre apparaisse. Le profil par défaut de chaque jeu est fixé après les tests.
- **Multi-écran** : best effort en v1. Seul l'écran du jeu change, les autres ne sont pas touchés.

### 2. Overlay crosshair (`src-tauri/src/overlay.rs`, `crosshair/`)
- **Fenêtre native Win32**, pas une WebView :
  - styles `WS_POPUP` + `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` ;
  - les clics passent à travers et elle n'apparaît pas dans alt-tab.
- **Rendu** : `tiny-skia` produit une image RGBA prémultipliée, copiée dans une `CreateDIBSection` puis affichée avec `UpdateLayeredWindow`. Rien n'est redessiné tant que rien ne change, donc coût nul en jeu.
- **Un seul moteur de rendu, en Rust.** L'aperçu de l'éditeur appelle une commande Tauri qui renvoie un PNG de ce même moteur. Ce qu'on voit dans l'éditeur est exactement ce qui s'affiche en jeu.
- **Position** : centre de la zone client de la fenêtre du jeu, plus un décalage X/Y réglable. On réaffirme `HWND_TOPMOST` à chaque changement de premier plan.
- **Visibilité** : affiché seulement si un jeu configuré est au premier plan. Raccourci on/off via `tauri-plugin-global-shortcut` (RegisterHotKey, sans risque anti-cheat).
- **Option anti-déformation** :
  - facteur X = `(mode_w / native_w) / (mode_h / native_h)` ;
  - exemple : en 1440x1080 sur un écran 1920x1080, X × 0,75 ;
  - le crosshair reste carré une fois étiré.
- **Limite connue** : invisible en plein écran exclusif. On recommande le borderless, et l'app l'indique par jeu.
- **Latence** :
  - Une fenêtre toujours au-dessus peut faire perdre au jeu le mode « Independent Flip ». On passerait alors en composition DWM : environ +1 frame et VRR cassé, sauf si Windows lui attribue un plan MPO.
  - À mesurer avec PresentMon (mode de présentation et latence), avec et sans overlay.
  - Désactiver le crosshair détruit la fenêtre overlay, ce qui garantit zéro impact.
- **Raccourcis** : `RegisterHotKey` peut être bloqué par les jeux qui lisent l'entrée brute avec `RIDEV_NOHOTKEYS`. Solution de repli : hook `WH_KEYBOARD_LL`, qui ne touche pas au process du jeu. Retour au natif d'urgence aussi accessible depuis le menu de l'icône près de l'horloge.
- **FACEIT et tournois** : avertissement dans l'app. Option activée par défaut : crosshair caché automatiquement si le client ou l'anti-cheat FACEIT tourne.

### 3. Modèle crosshair et imports (`src-tauri/src/crosshair/`)
- **`model.rs`** : modèle unique `Crosshair { kind: Lines | Image, color, opacity, outline{on,thickness,opacity}, center_dot{on,size,opacity}, inner{length,thickness,gap,opacity}, outer{…}, t_style, offset_xy, image_path, image_scale }`.
- **`valorant.rs`** : parse le code profil (`0;P;c;5;h;0;0l;4;0o;2;…`) vers le modèle.
  - Clés connues : `c` (couleur), `u` (hex), `h`/`t`/`o` (contour), `d`/`z`/`a` (point), `0l 0t 0o 0a` (lignes intérieures), `1l 1t 1o 1a` (lignes extérieures).
  - Clés de forme à gérer **obligatoirement** : `0b`/`1b` (lignes affichées ou masquées), `0v`/`0g` et `1v`/`1g` (longueur verticale séparée).
  - `0m`/`0f`/`1m`/`1f` (erreur de mouvement et de tir) : non reproductibles par un overlay fixe. On les ignore et l'app affiche une note « le crosshair externe ne bouge pas au tir ».
  - Les autres clés inconnues sont ignorées et listées dans un avertissement, jamais en silence.
- **`cs2.rs`** : décode `CSGO-XXXXX-…` avec le dictionnaire base 57 `ABCDEFGHJKLMNOPQRSTUVWXYZabcdefhijkmnopqrstuvwxyz23456789`, obtient les octets, puis les champs (gap, épaisseur, longueur, couleur RGB/alpha, contour, point, T, style).
  - On porte un décodeur open source de référence, avec des vecteurs de test.
  - La conversion unités CS2 → pixels dépend de la hauteur de résolution du jeu : à valider côte à côte.
- **Import PNG** : copié dans `%APPDATA%\REZ\crosshairs\`, avec taille et opacité réglables.
- **Partage** : export et import d'un code propre à l'app, `REZ1-<base64url(json)>`.
- **Bibliothèque** : liste nommée, un crosshair actif global, et un crosshair différent possible pour un jeu précis.

### 4. Profils de jeux (`src-tauri/src/profiles/`)
Chaque profil implémente `apply(res)` et `revert()` :
- **Sauvegarde** avant toute modification, dans `%APPDATA%\REZ\backups\<game>\`.
- **Restauration** si on retire le jeu.
- **Lecture seule** posée seulement si nécessaire. Prévenir l'utilisateur que les réglages qu'il change en jeu ne seront plus sauvegardés.

**Les formats changent avec les patchs : valider chaque clé en phase 3.**

| Jeu | Exe | Fichier modifié | Clés modifiées |
|---|---|---|---|
| Valorant | `VALORANT-Win64-Shipping.exe` | `%LOCALAPPDATA%\VALORANT\Saved\Config\<id>\Windows\GameUserSettings.ini` | `ResolutionSizeX/Y`, `LastUserConfirmedResolutionSizeX/Y`, `bShouldLetterbox=False`, `bLastConfirmedShouldLetterbox=False`, mode plein écran à valider |
| Fortnite | `FortniteClient-Win64-Shipping.exe` | `%LOCALAPPDATA%\FortniteGame\Saved\Config\WindowsClient\GameUserSettings.ini` | `ResolutionSizeX/Y`, `LastUserConfirmed…`, `DesiredScreenWidth/Height`, `LastUserConfirmedDesiredScreen…`, puis lecture seule |
| Apex | `r5apex.exe` / `r5apex_dx12.exe` | `%USERPROFILE%\Saved Games\Respawn\Apex\local\videoconfig.txt` | `setting.defaultres`, `setting.defaultresheight`, `setting.fullscreen`, `setting.nowindowborder`, puis lecture seule |
| CS2 | `cs2.exe` | `<Steam>\userdata\<id>\730\local\cfg\cs2_video.txt` | résolution et ratio / mode étiré (clés à valider) |
| Rust | `RustClient.exe` | profil générique : jeu en borderless, résolution en jeu = résolution du bureau | aucune modification de fichier |
| ROB RUN | `installdir` de l'appmanifest Steam 4262460, puis scan des .exe du dossier, avec confirmation par l'utilisateur. Une fois connu, le nom est codé en dur. | profil générique | aucune modification de fichier |

**Bandes noires de Rust / ROB RUN.** Si on règle 4:3 dans le jeu, il dessine une image 4:3 au milieu d'un écran 16:9, d'où les bandes. Avec l'app, quand le jeu est au premier plan, l'écran passe en 1440x1080. Le jeu en borderless voit alors un écran 4:3 et le remplit entièrement. Le GPU étire ensuite l'image sur tout l'écran physique. En alt-tab, l'écran repasse en natif et les autres fenêtres ne sont pas touchées.

Deux conditions :
- la mise à l'échelle GPU est réglée sur « plein écran », sinon le GPU remet des bandes (voir l'assistant) ;
- la résolution en jeu est égale ou supérieure à celle du bureau (le borderless prend celle du bureau).

- **Générique** : aucun fichier modifié. Il marche pour tout jeu en borderless, qui adopte la résolution du bureau. L'utilisateur peut ajouter n'importe quel .exe.
- **Détection des installations** (`games_detect.rs`) :
  - Steam : `libraryfolders.vdf` + `appmanifest_*.acf` ;
  - Epic : `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests\*.item` (JSON) ;
  - Riot : `%ProgramData%\Riot Games\RiotClientInstalls.json`.

### 5. Assistant de premier lancement : mise à l'échelle GPU
- **Détecter la marque du GPU** avec DXGI (`IDXGIAdapter::GetDesc`) et afficher le guide correspondant :
  - NVIDIA : « Ajuster la taille et la position du bureau → Plein écran, mise à l'échelle par le GPU » ;
  - AMD : Mise à l'échelle GPU ON + « Panneau complet » ;
  - Intel : « Étirer ».
- **Bouton « Tester »** :
  - passe 5 s en 1440x1080 avec une mire (un cercle et un cadre) ;
  - l'utilisateur répond « plein écran étiré » ou « bandes noires » ;
  - si bandes noires, renvoyer vers le guide ;
  - le retour au natif est automatique.
- **Écrans avec DSC.** Sur beaucoup d'écrans 1440p/4K à haute fréquence, NVIDIA désactive la mise à l'échelle GPU et les résolutions personnalisées.
  - Si l'option manque ou que le test échoue, l'assistant propose le repli : réglage « plein écran / étirer » dans le menu de l'écran.
  - Il prévient aussi du noir plus long (2 à 3 s) à chaque bascule.
  - Si 1440x1080 n'existe pas à la fréquence native, il propose les modes 4:3 disponibles à cette fréquence.

## Interface (React + TS + Tailwind, FR/EN avec react-i18next)

Style par défaut : sombre, sobre, orienté gaming. Le DESIGN.md présent (Poseidon Scans) n'est pas utilisé.

```
┌ Jeux ─ Crosshair ─ Réglages ───────────────────────┐
│ [VALORANT]  1440x1080 ▾  ● actif   profil OK  [⚙]  │
│ [CS2]       1280x960  ▾  ● actif   borderless [⚙]  │
│ [+ Ajouter un jeu]                                  │
└─────────────────────────────────────────────────────┘
Crosshair : [bibliothèque] | aperçu live | éditeur sliders | [Coller code] [Importer PNG]
Réglages  : raccourcis, démarrage Windows (off par défaut), langue, mises à jour
```

**Pages** :
- `src/pages/Games.tsx`
- `src/pages/Crosshair.tsx`
- `src/pages/Settings.tsx`
- `src/pages/Onboarding.tsx`

**Composants** :
- `CrosshairEditor.tsx`
- `CrosshairPreview.tsx`, qui affiche le PNG du moteur Rust

**Plugins Tauri** : `tray-icon`, `global-shortcut`, `autostart`, `single-instance`, `updater`, `dialog`, `fs`.

**Stockage** : `%APPDATA%\REZ\config.json`, qui contient les jeux, les profils, la bibliothèque de crosshairs et les réglages.

## Structure

```
package.json, vite.config.ts, src/ (UI React)
src-tauri/
  Cargo.toml, tauri.conf.json
  src/main.rs, lib.rs          # setup, tray, commandes Tauri
  src/display.rs               # trait DisplayApi + impl Win32
  src/watcher.rs               # hook premier plan + machine d'état
  src/overlay.rs               # fenêtre layered + UpdateLayeredWindow
  src/crosshair/{model,render,valorant,cs2,share}.rs
  src/profiles/{mod,generic,valorant,fortnite,apex,cs2}.rs
  src/games_detect.rs
  src/store.rs                 # config JSON, migrations
  tests/fixtures/              # ini/txt de jeux, codes crosshair
```

## Phases (chaque phase est livrable)

1. **Base** : scaffold Tauri, icône près de l'horloge, ajout manuel d'un .exe et choix de la résolution, watcher, changement et retour au natif, raccourci d'urgence. → Utilisable en borderless dès cette phase.
2. **Crosshair** : modèle, rendu, overlay, éditeur, bibliothèque, raccourci, anti-déformation.
3. **Imports** : codes Valorant et CS2, PNG, code de partage.
4. **Profils de jeux** : Valorant, Fortnite, Apex, CS2, avec sauvegarde et restauration, détection Steam/Epic/Riot.
5. **Assistant** : guide de mise à l'échelle GPU et mire de test.
6. **Distribution** :
   - installateur NSIS (bundler Tauri) ;
   - `tauri-plugin-updater` avec une clé de signature et GitHub Releases ;
   - signature de l'exe ;
   - page de téléchargement statique ;
   - FR/EN.

## Décisions prises

- **Nom** : REZ.
- **Open source** sur GitHub.
  - Licence MIT (mon choix).
  - Signature de l'exe gratuite via SignPath Foundation, à demander avant la phase 6.
  - CI GitHub Actions : build, tests et release.
  - Décodeurs de crosshair : on peut porter du code MIT ou Apache en citant la source. Le code GPL est réécrit de zéro.
- **Priorité étirement > crosshair.**
  - Chaque profil essaie d'abord le borderless, pour avoir l'étirement et le crosshair.
  - Si un jeu ne s'étire qu'en plein écran exclusif, le profil utilise ce mode. Il affiche alors « crosshair REZ indisponible dans ce jeu, utilise celui du jeu » dans la carte du jeu, et l'overlay reste désactivé pour ce jeu.
- **Pas de mode de secours** « copie étirée de la fenêtre » en v1.
- **Style de l'interface** (mon choix) : sombre, sobre, gaming. DESIGN.md (Poseidon Scans) n'est pas utilisé.

## Vérification

- **`cargo test`** :
  - parsers Valorant et CS2 sur des codes connus vers les réglages attendus ;
  - patchers de profils sur les fixtures `.ini`/`.txt` : sortie attendue, sauvegarde et restauration identiques à l'octet près ;
  - machine d'état avec `DisplayApi` mock : séquences focus, alt-tab, crash et fin de process vers les appels attendus.
- **`npm run tauri dev`** :
  - ajouter `notepad.exe` comme faux jeu ;
  - vérifier le passage en 1280x960 au focus, le retour au natif en alt-tab et le crosshair visible seulement sur notepad ;
  - tuer l'app (`taskkill /F`) pendant que le mode est actif : le retour au natif doit être immédiat, par Windows ou par le gardien ;
  - vérifier que les icônes du bureau et les fenêtres retrouvent leur place après une bascule aller-retour ;
  - verrouiller la session (Win+L) en jeu, puis revenir : le mode doit être réappliqué.
- **PresentMon** sur CS2 et Rust : mode de présentation et latence avec et sans overlay.
- **Matrice manuelle** :
  - chaque jeu v1 × (borderless, plein écran) × (NVIDIA, AMD, Intel si dispo) × (1 écran, 2 écrans) ;
  - vérifier l'étirement, l'alt-tab, le crosshair visible et la fréquence conservée.
- **`npm run tauri build`** : installer, puis publier une v0.0.2 et vérifier la mise à jour auto depuis la v0.0.1.
