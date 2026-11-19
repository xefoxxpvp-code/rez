# REZ — Phase 2 (Crosshair) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :**
- un crosshair par jeu, dessiné par REZ dans un overlay transparent, sans injection ;
- 5 presets créés au premier lancement ;
- un éditeur avec aperçu fidèle ;
- l'import de codes Valorant, CS2 et REZ, et d'images PNG.

**Architecture :**
- **Modèle et rendu** : un modèle `Crosshair` unique, rendu par `tiny-skia` dans `crosshair/render.rs`. Ce même rendu sert à l'overlay et aux aperçus de l'UI.
- **Décision d'affichage** : un `CrosshairDirector` pur, sans Win32, décide quoi montrer (`Show(spec)` / `Hide`). Le runtime l'alimente avec les mêmes entrées `Foreground` que l'engine.
- **Overlay** : la fenêtre (`win/overlay.rs`) vit sur son propre thread. Elle recalcule position et échelle 4 fois par seconde à partir de la fenêtre au premier plan.

**Tech Stack :** Rust (`tiny-skia` 0.11, `png`, `base64`, `windows` 0.62), Tauri 2, React/TS.

**Spec :**
- [docs/superpowers/specs/2026-11-16-rez-crosshair-design.md](../specs/2026-11-16-rez-crosshair-design.md) : prime en cas de conflit ;
- [docs/superpowers/specs/2026-11-16-rez-design.md](../specs/2026-11-16-rez-design.md) : sections 2 et 3.

## Global Constraints

- **Unités du modèle** : px à 1080p. Échelle de rendu = `hauteur du mode courant / 1080`.
- **Anti-déformation** :
  - facteur X = `(mode_w / native_w) / (mode_h / native_h)` ;
  - activée par défaut (`settings.crosshairUndistort = true`) ;
  - appliquée seulement à l'axe horizontal.
- **Fenêtre overlay** :
  - styles `WS_POPUP` + `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` ;
  - affichage par `UpdateLayeredWindow` avec un DIB 32 bpp prémultiplié ;
  - aucune injection, hook ou lecture mémoire dans un process de jeu.
- **Raccourci crosshair** : **Ctrl+Alt+Shift+X**, bascule pour la session, non persistée.
- **FACEIT** : process `faceit.exe`, `faceitclient.exe`, `faceitservice.exe` (insensible à la casse) → crosshair caché si `settings.hideCrosshairWithFaceit` (activé par défaut).
- **Défaut par jeu** : `crosshairId = None`.
- **Presets** : seedés une seule fois (`settings.crosshairsSeeded`).
- **Dossier des images** : `%APPDATA%\REZ\crosshairs\`.
- **Code REZ** : `REZ1-` + base64url sans padding du JSON d'un `Crosshair`, sans `id` ni `image`.
- **Hérité de la phase 1** : chaînes UI en français via `t()`, serde camelCase, `rtk` devant les commandes git, `cargo clippy --all-targets -- -D warnings` propre.

## Review Focus

1. **Le jeu change de taille après le changement de résolution.** Attendu : le crosshair se recentre en moins de 0,5 s. → test `geometry_centers_on_client_rect` (Task 3) ; vérification manuelle en Task 8.
2. **Une autre fenêtre passe au premier plan** (alt-tab, notification plein écran). Attendu : le crosshair disparaît. → test `hides_for_other_apps` (Task 3).
3. **Le crosshair utilisé par un jeu est supprimé.** Attendu : le jeu repasse sur « Aucun », sans crash ni overlay fantôme. → tests `deleting_crosshair_unassigns_games` (Task 1) et `rules_change_removing_crosshair_hides` (Task 3).
4. **Code collé invalide, tronqué, ou d'un autre jeu.** Attendu : un message clair, rien n'est créé. → tests `rejects_garbage` dans chaque parser (Task 6).
5. **PNG énorme ou corrompu.** Attendu : refus avec un message, et une image plus grande que 256 px est réduite à l'import. → test `png_import_rejects_corrupt` (Task 6).

---

## Structure

```
src-tauri/src/crosshair/mod.rs
src-tauri/src/crosshair/model.rs      # Crosshair, Lines, Outline, Dot, Circle
src-tauri/src/crosshair/presets.rs    # 5 presets
src-tauri/src/crosshair/render.rs     # tiny-skia → RgbaImage (premultiplied)
src-tauri/src/crosshair/director.rs   # pure show/hide decision + geometry
src-tauri/src/crosshair/valorant.rs   # Task 6
src-tauri/src/crosshair/cs2.rs        # Task 6
src-tauri/src/crosshair/share.rs      # Task 6 (REZ1 code)
src-tauri/src/imaging.rs              # encode_png / decode_png (moved from win/icon.rs)
src-tauri/src/win/overlay.rs          # layered window thread
src-tauri/src/win/process.rs          # is_any_running(&[names])
src/pages/Crosshair.tsx, src/components/CrosshairEditor.tsx, src/components/CrosshairPicker.tsx
```

---

### Task 1 : Modèle, config v2, presets

**Files :**
- Create : `crosshair/mod.rs`, `crosshair/model.rs`, `crosshair/presets.rs`
- Modify : `config.rs`, `store.rs` (seeding au chargement), `lib.rs`

**Interfaces :**
- Produces :
```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] #[serde(rename_all = "camelCase", default)]
pub struct Lines { pub enabled: bool, pub length: f32, pub vertical_length: Option<f32>, pub thickness: f32, pub gap: f32, pub opacity: f32 }
pub struct Outline { pub enabled: bool, pub thickness: f32, pub opacity: f32 }      // same derives
pub struct Dot { pub enabled: bool, pub size: f32, pub opacity: f32 }
pub struct Circle { pub enabled: bool, pub radius: f32, pub thickness: f32, pub opacity: f32 }
pub struct Crosshair { pub id: String, pub name: String, pub color: String /* #RRGGBB */, pub inner: Lines, pub outer: Lines,
                       pub t_style: bool, pub outline: Outline, pub dot: Dot, pub circle: Circle,
                       pub image: Option<String>, pub image_scale: f32, pub image_opacity: f32, pub offset_x: i32, pub offset_y: i32 }
impl Crosshair { pub fn rgb(&self) -> (u8, u8, u8) }   // invalid hex → white
pub fn presets() -> Vec<Crosshair>                      // ids "preset-classique", "preset-point", "preset-t", "preset-cercle", "preset-compact"
// config.rs
GameEntry { ..., pub crosshair_id: Option<String> }
Settings { ..., pub crosshairs_seeded: bool, pub crosshair_undistort: bool /* default true */, pub hide_crosshair_with_faceit: bool /* default true */ }
Config { ..., pub crosshairs: Vec<Crosshair> }
impl Config {
    pub fn seed_crosshairs(&mut self) -> bool;          // adds presets once; returns true if it changed the config
    pub fn upsert_crosshair(&mut self, c: Crosshair);
    pub fn delete_crosshair(&mut self, id: &str) -> Result<(), ConfigError>;  // unassigns games
    pub fn crosshair_rules(&self) -> Vec<CrosshairRule>; // enabled games with an existing crosshair
}
pub struct CrosshairRule { pub exe_name_lower: String, pub crosshair: Crosshair }   // in crosshair/director.rs
```
Les valeurs des presets sont celles du tableau de la spec. Valeurs non citées :
- opacités 1.0 ;
- contour : opacité 1.0, couleur noire ;
- `outer.enabled = false` ;
- `image_scale = 1.0`, `image_opacity = 1.0`.

`Settings::default()` met `crosshair_undistort` et `hide_crosshair_with_faceit` à `true`. Il faut donc un `impl Default` manuel, ou `#[serde(default = ...)]` sur ces deux champs.

- [ ] **Step 1 : Écrire les tests (RED)**
  - `presets_are_five_distinct_named_crosshairs` : 5 éléments, ids et noms uniques, noms exacts `["Classique","Point","T","Cercle","Compact"]`.
  - `seed_adds_presets_once` :
    - le 1er appel renvoie `true` et `crosshairs.len() == 5` ;
    - après avoir tout supprimé, le 2e appel renvoie `false` et `len() == 0`.
  - `deleting_crosshair_unassigns_games` : un jeu avec `crosshair_id = "preset-t"` → `delete_crosshair("preset-t")` → `crosshair_id == None`.
  - `crosshair_rules_resolve_ids` : jeu activé avec `"preset-point"` → 1 règle, `crosshair.name == "Point"`. Jeu désactivé ou id inconnu → aucune règle.
  - `v1_config_gets_crosshair_defaults` : le JSON de la phase 1 (sans champ crosshair) donne `crosshairs` vide, `crosshairs_seeded == false`, `crosshair_undistort == true`, `crosshair_id == None`.
  - `color_hex_parses` : `"#FF4FD8"` → `(255,79,216)`, `"bad"` → `(255,255,255)`.
- [ ] **Step 2** : `rtk cargo test --manifest-path src-tauri/Cargo.toml --lib` → FAIL (compilation).
- [ ] **Step 3 : Implémenter.** `store::load` n'appelle pas le seed. C'est `lib.rs` qui le fait au démarrage (`if config.seed_crosshairs() { store::save(..) }`).
- [ ] **Step 4** : tests verts, suite complète verte.
- [ ] **Step 5** : `rtk git commit -m "feat(crosshair): model, presets, config v2"`

---

### Task 2 : Rendu (`crosshair/render.rs`) + `imaging.rs`

**Files :**
- Create : `crosshair/render.rs`, `imaging.rs`
- Modify : `win/icon.rs` (utilise `imaging::encode_png`), `Cargo.toml` (`tiny-skia = "0.11"`)

**Interfaces :**
- Produces :
```rust
pub struct RgbaImage { pub width: u32, pub height: u32, pub pixels: Vec<u8> /* premultiplied RGBA */ }
pub fn render(c: &Crosshair, scale: f32, x_factor: f32, image_dir: &Path) -> RgbaImage;  // centered: center pixel = (width/2, height/2)
pub fn render_png(c: &Crosshair, scale: f32, image_dir: &Path) -> Vec<u8>;              // UI preview, x_factor 1
// imaging.rs
pub fn encode_png(rgba_straight: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String>;
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, String>;  // straight alpha out
pub fn unpremultiply(px: &mut [u8]);
```
**Règles de rendu :**
- Dimensions arrondies : `t = max(1, round(thickness*scale))`, `l = round(length*scale)`, `g = round(gap*scale)`.
- **Branches** :
  - droite : rect de `x = cx + g` à `cx + g + l` (×`x_factor` sur les distances horizontales), de `y = cy - t/2` sur la hauteur `t` ;
  - les 3 autres branches sont symétriques ;
  - en `t_style`, pas de branche haute.
- **Contour** : un rect noir élargi de `outline.thickness` de chaque côté, dessiné sous chaque forme.
- **Point** : carré `size`, centré.
- **Cercle** : trait d'anneau de rayon `radius` (horizontal ×`x_factor`, donc une ellipse si `x_factor ≠ 1`).
- **Image** : PNG décodé, mis à l'échelle `image_scale * scale` (×`x_factor` en largeur), centré. Image absente → rendu des formes seulement.
- **Canevas** : carré `2 * (extent + 2) + 1` px (impair, donc un vrai pixel central).

- [ ] **Step 1 : Tests (RED)**, sur le preset Classique à scale 1 et x_factor 1 :
  - `center_is_transparent_with_gap` ;
  - `right_arm_pixel_is_green` à `(cx+4, cy)` ;
  - `outline_pixel_is_black` à `(cx+3+6, cy)` (bout de la branche) ;
  - `t_style_has_no_top_arm` : `(cx, cy-5)` transparent pour le preset T ;
  - `dot_center_is_white` pour le preset Point ;
  - `x_factor_shrinks_horizontal_extent` : avec `x_factor` 0.5, `(cx+6, cy)` transparent et `(cx, cy+6)` vert ;
  - `canvas_is_odd_and_centered` ;
  - `png_roundtrip` (imaging).
- [ ] **Step 2** : FAIL. **Step 3** : implémenter. **Step 4** : vert. **Step 5** : commit `feat(crosshair): tiny-skia renderer`.

---

### Task 3 : Director + géométrie (purs)

**Files :**
- Create : `crosshair/director.rs`

**Interfaces :**
```rust
pub struct OverlaySpec { pub pid: u32, pub crosshair: Crosshair, pub native: (u32, u32), pub undistort: bool }
#[derive(Debug, PartialEq)] pub enum OverlayCommand { Show(OverlaySpec), Hide }   // OverlaySpec: Clone, Debug, PartialEq
pub struct CrosshairDirector { /* rules, visible, undistort, hide_with_faceit, shown: Option<OverlaySpec>, last_fg: Option<(u32, String, (u32,u32))> */ }
impl CrosshairDirector {
    pub fn new(rules: Vec<CrosshairRule>, undistort: bool, hide_with_faceit: bool) -> Self;
    pub fn on_foreground(&mut self, pid: u32, exe_name: &str, native: (u32, u32), faceit_running: bool) -> Option<OverlayCommand>;
    pub fn on_rules(&mut self, rules: Vec<CrosshairRule>, undistort: bool, hide_with_faceit: bool, faceit_running: bool) -> Option<OverlayCommand>;
    pub fn toggle(&mut self, faceit_running: bool) -> Option<OverlayCommand>;
}
pub struct Geometry { pub scale: f32, pub x_factor: f32 }
pub fn geometry(mode: (u32, u32), native: (u32, u32), undistort: bool) -> Geometry;
pub fn client_center(left: i32, top: i32, right: i32, bottom: i32, offset: (i32, i32)) -> (i32, i32);
```
`None` = rien ne change (pas de commande redondante).

- [ ] **Step 1 : Tests (RED)**
  - `shows_for_configured_game` : `Some(Show(spec))` avec le bon pid et le bon crosshair. `exe` en majuscules : même résultat.
  - `hides_for_other_apps` : jeu, puis `chrome.exe` → `Some(Hide)` ; un 2e `chrome.exe` → `None`.
  - `toggle_hides_and_shows_back` : jeu affiché → `toggle` → `Hide` → `toggle` → `Show`.
  - `faceit_hides_when_enabled` : `faceit_running = true` → pas de `Show`. Avec `hide_with_faceit = false` → `Show`.
  - `rules_change_updates_shown_crosshair` : en jeu, nouveau crosshair dans les règles → `Show` avec le nouveau.
  - `rules_change_removing_crosshair_hides` : en jeu, règles vides → `Hide`.
  - `empty_exe_hides`.
  - `geometry_stretched_undistorts` :
    - `(1440,1080)` vs `(1920,1080)` → `scale 1.0`, `x 0.75` ;
    - `(1280,960)` → `scale ≈ 0.8889`, `x 0.75` ;
    - `undistort = false` → `x 1.0`.
  - `geometry_centers_on_client_rect` : `client_center(0,0,1440,1080,(0,0)) == (720,540)`, et avec `offset (2,-3)` → `(722,537)`.
- [ ] **Steps 2-5** : FAIL → implémenter → vert → commit `feat(crosshair): display director and geometry`.

---

### Task 4 : Overlay Win32 + détection de process

**Files :**
- Create : `win/overlay.rs`, `win/process.rs`, `examples/overlay_check.rs`

**Interfaces :**
```rust
pub trait OverlayApi: Send { fn apply(&mut self, cmd: OverlayCommand); }   // in runtime.rs
pub struct Win32Overlay;  impl Win32Overlay { pub fn spawn(image_dir: PathBuf) -> Self }  impl OverlayApi
pub fn is_any_running(names_lower: &[&str]) -> bool;   // Toolhelp snapshot
```
**Comportement du thread overlay** :
- il possède sa fenêtre et sa boucle de messages, et reçoit les commandes par `mpsc` ;
- un timer de 250 ms (`SetTimer`) déclenche la mise à jour ;
- **si `Show(spec)`** :
  - fenêtre au premier plan → pid ; si différent de `spec.pid`, la fenêtre est cachée ;
  - sinon, `GetClientRect` + `ClientToScreen` donnent le centre ;
  - le mode courant de l'écran de la fenêtre donne la géométrie ;
  - on rend et on appelle `UpdateLayeredWindow` seulement si quelque chose a changé (centre, taille ou crosshair) ;
  - on réaffirme `HWND_TOPMOST` à chaque nouveau `Show`.
- **si `Hide`** : `ShowWindow(SW_HIDE)`.

- [ ] **Step 1** : implémenter, avec un rendu à partir de `render()` converti en BGRA prémultiplié.
- [ ] **Step 2** : `examples/overlay_check.rs` affiche le preset Classique centré sur le Bloc-notes pendant 10 s. **À lancer seulement avec l'accord de l'utilisateur, qui ne doit pas être en train de jouer.**
- [ ] **Step 3** : clippy propre, puis commit `feat(win): crosshair overlay window and process detection`.

---

### Task 5 : Intégration runtime + commandes + raccourci

**Files :**
- Modify : `runtime.rs`, `lib.rs`, `commands.rs`

**Interfaces :**
- Inputs ajoutés :
  - `Input::CrosshairRulesChanged { rules: Vec<CrosshairRule>, undistort: bool, hide_with_faceit: bool }` ;
  - `Input::ToggleCrosshair`.
- `Runtime::with_crosshair(self, overlay: Box<dyn OverlayApi>, faceit_running: Box<dyn Fn() -> bool + Send>) -> Self`. Par défaut, l'overlay ne fait rien.
- Sur `Foreground`, le runtime appelle le director avec le natif du moniteur (cache existant) et applique la commande.
- `mutate` (commands) envoie aussi `CrosshairRulesChanged`.
- Commandes :
  - `list_crosshairs() -> Vec<Crosshair>` ;
  - `save_crosshair(c) -> Crosshair` (crée un id `custom-<unix_ms>` si vide) ;
  - `delete_crosshair(id)` ;
  - `preview_crosshair(c) -> String` (data URL, scale 2.0 pour la netteté) ;
  - `set_crosshair_settings(undistort, hide_with_faceit)`.
- Raccourci Ctrl+Alt+Shift+X → `Input::ToggleCrosshair`.

- [ ] **Step 1 : Tests runtime (RED)**, avec un `MockOverlay` qui enregistre les commandes :
  - `runtime_shows_crosshair_for_game` ;
  - `runtime_hides_crosshair_on_alt_tab` ;
  - `toggle_input_hides_crosshair` ;
  - `faceit_probe_hides_crosshair`.
- [ ] **Steps 2-5** : FAIL → implémenter → vert → commit `feat(crosshair): runtime integration, commands, hotkey`.

---

### Task 6 : Imports (codes Valorant, CS2, REZ ; PNG)

**Files :**
- Create : `crosshair/valorant.rs`, `crosshair/cs2.rs`, `crosshair/share.rs`
- Modify : `commands.rs`

**Interfaces :**
```rust
pub struct Imported { pub crosshair: Crosshair, pub notes: Vec<String> }   // notes: ignored dynamic settings, unknown keys
pub fn import_code(code: &str) -> Result<Imported, String>;  // dispatch: "REZ1-" / "CSGO-" / starts with "0;"
pub fn valorant::parse(code: &str) -> Result<Imported, String>;
pub fn cs2::parse(code: &str) -> Result<Imported, String>;
pub fn share::encode(c: &Crosshair) -> String;  pub fn share::decode(code: &str) -> Result<Crosshair, String>;
#[tauri::command] import_crosshair_code(code) -> Imported (not saved; UI saves);  import_crosshair_image(path) -> Crosshair (copied + saved)
```
Les tables de clés, les valeurs par défaut, le décodage CS2 et les vecteurs de test exacts sont dans l'annexe A, ajoutée quand la recherche est terminée.

- [ ] **Step 1 : Tests (RED)**, avec les vecteurs de l'annexe A, plus :
  - `rejects_garbage` dans chaque parser ;
  - `share_roundtrip` ;
  - `png_import_rejects_corrupt` ;
  - `png_import_downscales_over_256`.
- [ ] **Steps 2-5** : FAIL → implémenter → vert → commit `feat(crosshair): Valorant, CS2, REZ codes and PNG import`.

---

### Task 7 : Interface

**Files :**
- Create : `src/pages/Crosshair.tsx`, `src/components/CrosshairEditor.tsx`, `src/components/CrosshairPicker.tsx`, `src/lib/crosshair.ts` (+ test)
- Modify : `App.tsx` (onglet), `GameCard.tsx` (sélecteur), `Settings.tsx`, `types.ts`, `api.ts`, `fr.json`

**Comportement :**
- **Onglet « Crosshair »** :
  - à gauche, la liste en vignettes ;
  - à droite, l'éditeur ;
  - l'aperçu passe par `preview_crosshair`, avec 60 ms d'anti-rebond ;
  - l'enregistrement est automatique, 400 ms après la dernière modification ;
  - curseurs : longueur, épaisseur, écart, opacité, contour, point, cercle ;
  - interrupteurs : branches, T, contour, point, cercle ;
  - couleur : 8 pastilles + choix libre.
- **Boutons** : Nouveau, Dupliquer, Supprimer (avec confirmation), Coller un code (dialogue + notes), Importer une image, Copier le code REZ.
- **`GameCard`** : `CrosshairPicker` en liste déroulante avec vignettes, dont « Aucun ».
- **Réglages** : interrupteurs anti-déformation et FACEIT, plus le raccourci X.
- **Test vitest** `detectCodeKind` : `"REZ1-…"` → `rez`, `"CSGO-…"` → `cs2`, `"0;P;…"` → `valorant`, sinon `null`. Il sert à l'étiquette du dialogue.

- [ ] Steps : test RED → implémenter → `tsc`, `vitest`, `vite build` verts → commit `feat(ui): crosshair page, editor, per-game picker`.

---

### Task 8 : Vérification

- [ ] **Suites automatiques** : `cargo test`, clippy, vitest, tsc.
- [ ] **Vérifications à l'écran, seulement avec l'accord de l'utilisateur et hors partie** :
  - le crosshair Classique s'affiche sur le Bloc-notes ;
  - il disparaît en alt-tab ;
  - Ctrl+Alt+Shift+X le cache et le réaffiche ;
  - en 1440x1080 étiré, il reste carré à l'écran.
- [ ] **Build** : `CARGO_TARGET_DIR=src-tauri/target-dist npm run tauri build` → installateur.
- [ ] **Note** : `docs/superpowers/notes/2026-11-16-phase2-verification.md`, puis commit.

---

### Task 9 : Mise à jour dans l'app (demande du 2026-11-19)

L'utilisateur met à jour REZ depuis l'app, sans réinstaller.

**Files :**
- Modify : `Cargo.toml` (`tauri-plugin-updater = "2"`, `tauri-plugin-process = "2"`), `lib.rs`, `tauri.conf.json`, `capabilities/default.json`, `package.json`
- Create : `src/components/UpdateBanner.tsx`, `.github/workflows/release.yml`

**Décisions :**
- **Clé de signature** :
  - générée par `npx tauri signer generate --ci -w %USERPROFILE%\.tauri\rez-updater.key` ;
  - la clé privée ne va **jamais** dans le dépôt ;
  - la clé publique va dans `tauri.conf.json > plugins.updater.pubkey`.
- **Endpoint** : `https://github.com/xefoxxpvp-code/rez/releases/latest/download/latest.json`. À confirmer par l'utilisateur avant toute création de dépôt ou publication.
- **Bundle** : `createUpdaterArtifacts: true`. Updater en `windows.installMode: "passive"`.
- **Avant l'install** : `on_before_exit` envoie `Shutdown` au runtime, qui remet le natif et vide la disposition du bureau en attente, puis attend l'ack (2 s max).
- **Interface** :
  - au démarrage, `check()` ; si une version est disponible, une bannière « Mise à jour X disponible » avec un bouton « Mettre à jour » (progression, puis `relaunch()`) ;
  - dans Réglages, la version actuelle et un bouton « Rechercher une mise à jour ».
- **CI** :
  - `release.yml` se déclenche sur un tag `v*` ;
  - il utilise `tauri-apps/tauri-action@v0` avec `includeUpdaterJson`, sur `windows-latest` ;
  - secrets requis : `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (vide).
- **Version** : 0.2.0 dans `tauri.conf.json`, `Cargo.toml` et `package.json`.

- [ ] **Step 1** : `vitest` sur `updateLabel(version)` → `"Mise à jour 0.3.0 disponible"`. RED puis GREEN.
- [ ] **Step 2** : plugins, configuration, bannière, réglages, workflow.
- [ ] **Step 3** : `tauri build` produit `REZ_0.2.0_x64-setup.exe` + `.sig`. Vérifier que `latest.json` peut être généré : le workflow le produit, en local seul le `.sig` est vérifié.
- [ ] **Step 4** : commit `feat(app): in-app updates`.

---

## Annexe A : formats d'import (recherche du 2026-11-19)

Les références sous licence MIT peuvent être portées en citant la source (`csgo-sharecode` d'akiver, `genesy/crosshair-codes`, `sciencejiho/valorant-crosshair`). Les dépôts sans licence peuvent être lus, mais pas copiés.

### CS2 : conteneur commun
- **Dictionnaire base 57** : `ABCDEFGHJKLMNOPQRSTUVWXYZabcdefhijkmnopqrstuvwxyz23456789`.
- **Décodage** :
  1. on retire le préfixe (`CSGO-` ou `CS`) et les tirets ;
  2. on **inverse** les caractères ;
  3. `big = big*57 + idx` ;
  4. on écrit `big` en big-endian sur `n` octets ;
  5. si `big ≥ 2^(8n)`, on refuse.
- **Checksum** : `b0 == (somme b1..) % 256`. La version est dans `b1`.
- **Formats** :
  - `CSGO-` : 25 caractères → 18 octets. On accepte la version 1. Les versions 3 et 4 sont refusées avec un message.
  - `CS` : 44 caractères → 32 octets, version 1.

### CS2 legacy-v1 (`CSGO-`)

**Champs** :

| Champ | Lecture |
|---|---|
| gap | `i8(b2)/10` |
| épaisseur du contour | `b3/2` |
| RGBA | `b4..b7` |
| index couleur | `b10&7` |
| contour | `b10&8` |
| épaisseur | `b12/10` |
| point | `b13&0x10` |
| alpha activé | `b13&0x40` |
| style T | `b13&0x80` |
| style | `(b13&0x0E)>>1` |
| longueur | `(b14 \| (b15&0x1F)<<8)/10` |

**Couleurs prédéfinies** : 0 `#FA3232`, 1 `#32FA32`, 2 `#FAFA32`, 3 `#3232FA`, 4 `#32FAFA`, 5 = RGB personnalisé.

**Vers le modèle** (px à 1080p, `s = 1080/480 = 2.25`) :
- `W = max(1, roundEven(épaisseur·s))` ;
- `L = roundEven(longueur·s)` ;
- `d = trunc(gap + 4)` (gap non mis à l'échelle) ;
- gap du modèle = `⌊W/2⌋ + d` ;
- point : taille `W` ;
- contour : épaisseur `b3/2` px ;
- opacité : `alpha/255` si alpha activé, sinon 1.

**Vecteurs** :
1. `CSGO-Cn37R-YE7vo-pLCAL-aURmZ-z6zkG`
   - octets `69 01 F3 04 AF 51 D5 89 06 1E 6D 54 0C F4 2E 00 00 00` ;
   - gap −1.3, contour 2, RGBA 175/81/213/137, index 5, contour activé, épaisseur 1.2 ;
   - point, alpha et T activés ; style 2 ; longueur 4.6.
2. `CSGO-LibdP-VCVEd-ESayK-rSivi-2UBtG`
   - gap 1, contour 1, RGBA 50/250/50/200, index 1, contour désactivé, épaisseur 0.5 ;
   - tous les drapeaux désactivés ; style 3 ; longueur 5.

### CS2 cs2-v1 (`CS…`, format actuel)

**Champs** :

| Champ | Lecture |
|---|---|
| hauteur d'écran `sh` | `u16le(b2,b3)`. 0 → invalide, `< 240` → 240 |
| style | `b4&0x1F` |
| point | `b4&0x40` |
| T | `b4&0x80` |
| RGBA | `b5..b8` |
| couleur du contour | `b9..b12` |
| épaisseur | `b13&0x3F` |
| mode de contour | `b13>>6` (0 aucun, 1 plein, 2 demi) |
| gap | `i16le(b14,b15)` |
| longueur | `b16` |

**Vers le modèle** : `r = 1080/sh` ; longueur, épaisseur et gap = `valeur·r` ; contour 1 px si le mode est différent de 0.

**Vecteurs** :
1. `CSvbPubOq37zTGqtsPTP5QTrp5CB4xFXiKRLfzJsm49ZRe` : sh 768, style 2, point, RGBA 255/0/0/255, contour 0, épaisseur 1, gap 0, longueur 5.
2. `CSAB6Oby2JUDOrBRy4jRcsR6JOQdhbszKF9GuKQGhKVwAc` : sh 1440, style 9, T, contour 2, RGBA 10/20/30/200, épaisseur 12, gap −7, longueur 255.

### Valorant (`0;…`)

**Structure** :
- les jetons sont séparés par `;` ;
- le premier jeton vaut `0` ;
- les sections sont `P` (lu), `A` et `S` (ignorés) ;
- les globaux `s`, `p`, `c` (avant `P`) sont ignorés.

**Défauts dans `P`** :
- couleur `c` = 0 (8 = personnalisée, lue dans `u` en RRGGBBAA) ;
- contour : `h` = 1, `t` = 1, `o` = 0.5 ;
- point : `d` = 0, `z` = 2, `a` = 1 ;
- lignes internes (préfixe `0`) : `b` 1, `t` 2, `l` 6, `g` 0, `v` 6, `o` 3, `a` 0.8 ;
- lignes externes (préfixe `1`) : `b` 1, `t` 2, `l` 2, `g` 0, `v` 2, `o` 10, `a` 0.35.

**Palette** : `#FFFFFF`, `#00FF00`, `#7FFF00`, `#DFFF00`, `#FFFF00`, `#00FFFF`, `#FF00FF`, `#FF0000`.

**Clés dynamiques ignorées** : `0m 0s 0f 0e 1m 1s 1f 1e`, et dans `P` : `f`, `m`, `s`. Si les lignes sont affichées, on ajoute la note « crosshair fixe : l'écartement au tir et en mouvement n'est pas reproduit ». Les valeurs sont prises en px (consensus de la communauté).

**Vecteurs** :
1. `0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0` : cyan, sans contour, interne longueur 4 / vertical `None` / épaisseur 2 / gap 2 / opacité 1, externe désactivé.
2. `0;s;1;P;c;8;u;FF00AAFF;d;1;z;3;0g;1;0v;8;1b;0;S;c;1` :
   - `#FF00AA`, point taille 3 ;
   - interne longueur 6 / vertical 8 / épaisseur 2 / gap 3 / opacité 0.8 ;
   - contour activé, 1 px, opacité 0.5 ;
   - externe désactivé.
