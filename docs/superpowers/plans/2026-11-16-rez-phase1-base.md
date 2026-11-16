# REZ — Phase 1 (Base) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** une app Windows Tauri « REZ » qui :
- tourne avec une icône près de l'horloge ;
- permet d'ajouter un .exe et de lui associer une résolution ;
- passe l'écran du jeu dans cette résolution (mode temporaire, fréquence native) quand le jeu est au premier plan ;
- revient en natif 250 ms après qu'on le quitte, ou à la fin du jeu, ou à la fermeture de l'app.

À la fin de cette phase, l'app est utilisable avec les jeux en borderless.

**Architecture :** un cœur Rust pur et testable (`engine.rs`, machine d'état sans Win32) piloté par un `runtime.rs` qui exécute ses actions via deux traits :
- `DisplayApi` (changement de mode) ;
- `LayoutApi` (icônes et fenêtres).

Les implémentations Win32 sont isolées dans `src-tauri/src/win/`, et les tests utilisent des mocks. L'UI React n'est qu'une vue de la config, via les commandes Tauri.

**Tech Stack :** Tauri 2.x (stable, pas 3.0-alpha), Rust 1.97 (MSVC), crate `windows` 0.62, React 18 + TypeScript + Vite, Tailwind CSS v4, react-i18next, Vitest.

**Spec :** [docs/superpowers/specs/2026-11-16-rez-design.md](../specs/2026-11-16-rez-design.md). Cette phase couvre la section « 1. Résolution par jeu » et la phase 1 du design. Les phases 2 à 6 auront chacune leur plan.

**Hors de cette phase :**
- option « appliquer dès le lancement » : phase 4, réglée par profil après tests ;
- repli des raccourcis via `WH_KEYBOARD_LL` : phase 2, avec les raccourcis du crosshair ;
- guide « résolution manquante » : phase 5, assistant.

## Global Constraints

- Windows 10/11 x64 uniquement, cible `x86_64-pc-windows-msvc`.
- `tauri = "2"` et plugins `"2"`. Jamais de version 3.0-alpha.
- Nom produit `REZ`, identifiant `com.rez.app`, licence MIT.
- **Jamais d'écriture du mode d'affichage dans le registre** :
  - application : `ChangeDisplaySettingsExW(device, &devmode, CDS_FULLSCREEN)` ;
  - retour : `ChangeDisplaySettingsExW(device, NULL, 0)`.
- **Aucune injection, lecture mémoire ou hook dans un process de jeu.** Seul `SetWinEventHook` avec `WINEVENT_OUTOFCONTEXT` est autorisé.
- Fréquence native conservée lors du changement de mode.
- Délai avant retour au natif : `LEAVE_DEBOUNCE = 250 ms`.
- Données : `%APPDATA%\REZ\config.json`. Logs : dossier log de Tauri (`tauri-plugin-log`).
- Démarrage avec Windows **désactivé par défaut**. Quand il est actif, l'app démarre avec `--minimized`, sans fenêtre.
- Raccourci d'urgence (retour natif) : **Ctrl+Alt+Shift+R**.
- Chaînes d'interface en français, toutes via `t()` (`src/i18n/fr.json`). L'anglais arrive en phase 6.
- Structures IPC : `#[serde(rename_all = "camelCase")]`. Enums : `#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]`.
- Identifiants et commentaires du code en anglais.
- Commandes git préfixées par `rtk` (préférence de l'utilisateur).

## Review Focus

1. **Exe illisible au premier plan.** Le nom de l'exe du process au premier plan ne peut pas être lu (process protégé ou élevé, anti-cheat). Attendu : traité comme « pas un jeu », pas de crash, warning loggé. → test `runtime_ignores_unreadable_exe` (Task 7).
2. **Mode refusé par le pilote** (`DISP_CHANGE_BADMODE`, etc.). Attendu : statut `Error` affiché, l'app continue, et le retour natif est toujours tenté à la sortie. → test `runtime_reports_apply_error` (Task 7).
3. **`config.json` corrompu ou d'une ancienne version.** Attendu :
   - fichier corrompu : sauvegardé en `config.json.corrupt-<unix>`, puis config par défaut ;
   - champs manquants : valeurs par défaut.
   → tests `corrupt_file_is_backed_up_and_default_returned` et `missing_fields_get_defaults` (Task 4).
4. **Même jeu ajouté deux fois** (casse différente, autre chemin vers le même nom d'exe). Attendu : erreur « déjà dans la liste », et correspondance insensible à la casse au premier plan. → tests `add_same_exe_twice_fails_case_insensitive` (Task 4) et `exe_match_is_case_insensitive` (Task 5).
5. **Jeu retiré ou désactivé pendant qu'il est étiré.** Attendu : retour natif immédiat. → test `rules_changed_removing_active_game_restores` (Task 5).

---

## Structure des fichiers

```
LICENSE, README.md, .gitignore
package.json, vite.config.ts, index.html, tsconfig*.json
src/main.tsx, App.tsx, index.css
src/i18n/index.ts, src/i18n/fr.json
src/lib/types.ts        # miroir TS des structs IPC
src/lib/api.ts          # wrappers invoke/listen
src/lib/format.ts (+ format.test.ts)
src/pages/Games.tsx, src/pages/Settings.tsx
src/components/GameCard.tsx, src/components/StatusBar.tsx
src-tauri/Cargo.toml, tauri.conf.json, capabilities/default.json
src-tauri/src/main.rs           # aiguillage --guardian / run()
src-tauri/src/lib.rs            # builder Tauri, état, events, exit
src-tauri/src/display.rs        # DisplayMode, DisplayApi, pick_mode, default_resolution
src-tauri/src/config.rs         # Config, GameEntry, Settings, rules()
src-tauri/src/store.rs          # load/save atomique
src-tauri/src/engine.rs         # machine d'état pure
src-tauri/src/runtime.rs        # Input, LayoutApi, Runtime (thread)
src-tauri/src/commands.rs       # commandes Tauri
src-tauri/src/tray.rs           # icône près de l'horloge, menu
src-tauri/src/win/mod.rs
src-tauri/src/win/display.rs        # Win32Display, restore_all()
src-tauri/src/win/events.rs         # hook premier plan, fenêtre cachée, surveillance process
src-tauri/src/win/desktop_layout.rs # Win32Layout (icônes + fenêtres)
src-tauri/src/win/guardian.rs       # seulement si le spike de la Task 3 l'exige
src-tauri/examples/{list_modes,cds_spike,watch_events}.rs
```

---

### Task 1 : Scaffold du dépôt et de l'app Tauri

**Files :**
- Create : tout le scaffold `create-tauri-app` (template `react-ts`, npm), `LICENSE` (MIT, 2026, « REZ contributors »), `README.md` (3 lignes : quoi, statut, licence)
- Modify : `src-tauri/tauri.conf.json`, `.gitignore`

**Interfaces :**
- Produces : projet qui compile, avec `src-tauri/src/lib.rs::run()` et `src-tauri/src/main.rs`.

- [ ] **Step 1 : Scaffold**
  - Commande : `npm create tauri-app@latest rez -- --template react-ts --manager npm --identifier com.rez.app -y`.
  - La racine du projet = la racine du workspace. Si l'outil refuse le dossier non vide, scaffolder dans le scratchpad puis déplacer le contenu de `rez/` à la racine.
  - Ne pas toucher `DESIGN.md`.
- [ ] **Step 2 : Configurer `tauri.conf.json`**
  - `productName: "REZ"`, `identifier: "com.rez.app"`.
  - `app.windows: []` : la fenêtre est créée en Rust en Task 9.
  - `bundle.targets: ["nsis"]`.
  - `"tauri" = "2"` dans `Cargo.toml`.
- [ ] **Step 3 : `rtk git init`**, puis compléter `.gitignore` : `node_modules/`, `dist/`, `src-tauri/target/`.
- [ ] **Step 4 : Vérifier**
  - `rtk npm install` puis `rtk cargo test --manifest-path src-tauri/Cargo.toml` : attendu, compile et « 0 tests ».
  - `npm run tauri dev` : attendu, compile sans erreur. Aucune fenêtre ne s'ouvre, c'est normal avec `windows: []`. Arrêter avec Ctrl+C.
- [ ] **Step 5 : Commit**
  `rtk git add -A && rtk git commit -m "chore: scaffold REZ (Tauri 2 + React TS), MIT license, design spec"`

---

### Task 2 : Modes d'affichage (`display.rs` + `win/display.rs`)

**Files :**
- Create : `src-tauri/src/display.rs`, `src-tauri/src/win/mod.rs`, `src-tauri/src/win/display.rs`, `src-tauri/examples/list_modes.rs`
- Modify : `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` (déclarer `pub mod display; pub mod win;`)

**Interfaces :**
- Produces (`display.rs`) :
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode { pub width: u32, pub height: u32, pub refresh_hz: u32 }

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo { pub device: String, pub label: String, pub primary: bool }

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("mode refusé par Windows (code {0})")] Rejected(i32),
    #[error("{0}")] Win32(String),
}

pub trait DisplayApi: Send + Sync {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>, DisplayError>;
    fn list_modes(&self, device: &str) -> Result<Vec<DisplayMode>, DisplayError>;
    fn registry_mode(&self, device: &str) -> Result<DisplayMode, DisplayError>;
    fn current_mode(&self, device: &str) -> Result<DisplayMode, DisplayError>;
    fn apply_temporary(&self, device: &str, mode: DisplayMode) -> Result<(), DisplayError>;
    fn restore(&self, device: &str) -> Result<(), DisplayError>;
}

pub fn pick_mode(modes: &[DisplayMode], width: u32, height: u32, native_hz: u32) -> Option<DisplayMode>;
pub fn default_resolution(modes: &[DisplayMode], native: DisplayMode) -> (u32, u32);
pub fn distinct_resolutions(modes: &[DisplayMode]) -> Vec<(u32, u32)>; // dédupliqué, trié par largeur puis hauteur décroissantes
```
- Produces (`win/display.rs`) : `pub struct Win32Display;` (impl `DisplayApi`) et `pub fn restore_all()`, qui restaure chaque écran attaché au bureau, sans panique ni erreur remontée.

- [ ] **Step 1 : Dépendances**
  - `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `thiserror = "2"`, `log = "0.4"`.
  - `windows = "0.62"` avec les features `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Accessibility`, `Win32_System_Threading`, `Win32_System_RemoteDesktop`, `Win32_System_Com`, `Win32_System_Variant`, `Win32_System_Ole`, `Win32_UI_Shell`, `Win32_UI_Shell_Common`.
  - Dev : `tempfile = "3"`.
- [ ] **Step 2 : Écrire les tests (dans `display.rs`, `#[cfg(test)]`)**
```rust
fn m(w: u32, h: u32, hz: u32) -> DisplayMode { DisplayMode { width: w, height: h, refresh_hz: hz } }

#[test] fn pick_mode_keeps_native_refresh() {
    let modes = [m(1920,1080,240), m(1440,1080,60), m(1440,1080,240)];
    assert_eq!(pick_mode(&modes, 1440, 1080, 240), Some(m(1440,1080,240)));
}
#[test] fn pick_mode_falls_back_to_highest_refresh() {
    let modes = [m(1440,1080,60), m(1440,1080,120)];
    assert_eq!(pick_mode(&modes, 1440, 1080, 240), Some(m(1440,1080,120)));
}
#[test] fn pick_mode_none_when_resolution_missing() {
    assert_eq!(pick_mode(&[m(1920,1080,144)], 1440, 1080, 144), None);
}
#[test] fn default_resolution_prefers_1440x1080() {
    let modes = [m(1920,1080,144), m(1440,1080,144), m(1280,960,144)];
    assert_eq!(default_resolution(&modes, m(1920,1080,144)), (1440,1080));
}
#[test] fn default_resolution_largest_4_3_otherwise() {
    let modes = [m(1920,1080,144), m(1280,960,144), m(1024,768,144)];
    assert_eq!(default_resolution(&modes, m(1920,1080,144)), (1280,960));
}
#[test] fn default_resolution_native_when_no_4_3() {
    assert_eq!(default_resolution(&[m(1920,1080,144)], m(1920,1080,144)), (1920,1080));
}
#[test] fn distinct_resolutions_dedups_and_sorts() {
    let modes = [m(1280,960,60), m(1920,1080,60), m(1280,960,144)];
    assert_eq!(distinct_resolutions(&modes), vec![(1920,1080), (1280,960)]);
}
```
- [ ] **Step 3 : Lancer les tests et vérifier qu'ils échouent**
  - Commande : `rtk cargo test --manifest-path src-tauri/Cargo.toml display`.
  - Attendu : FAIL, fonctions non définies.
- [ ] **Step 4 : Implémenter les fonctions pures.** Une résolution est 4:3 si `w * 3 == h * 4`.
- [ ] **Step 5 : Implémenter `Win32Display`**
  - **Écrans** : `EnumDisplayDevicesW`, en ne gardant que `DISPLAY_DEVICE_ATTACHED_TO_DESKTOP`. `primary` vient de `DISPLAY_DEVICE_PRIMARY_DEVICE`. `label` = `DeviceString` du moniteur, obtenu par un second `EnumDisplayDevicesW(device, 0)`.
  - **Modes** : boucle `EnumDisplaySettingsExW(device, i, …)`, en ne gardant que `dmBitsPerPel == 32`.
  - **Mode du registre** : `ENUM_REGISTRY_SETTINGS`. **Mode courant** : `ENUM_CURRENT_SETTINGS`.
  - **`apply_temporary`** :
    - remplir `dmPelsWidth`, `dmPelsHeight` et `dmDisplayFrequency` ;
    - ajouter `dmDisplayFixedOutput = DMDFO_STRETCH` et les champs `DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_DISPLAYFIXEDOUTPUT` ;
    - si le résultat n'est pas `DISP_CHANGE_SUCCESSFUL`, réessayer sans `DM_DISPLAYFIXEDOUTPUT` ;
    - sinon renvoyer `Rejected(code)`.
  - **`restore`** : `ChangeDisplaySettingsExW(device, None, None, CDS_TYPE(0), None)`.
- [ ] **Step 6 : `examples/list_modes.rs`.** Affiche chaque écran (device, label, primary), son mode registre et ses résolutions distinctes.
- [ ] **Step 7 : Vérifier**
  - `rtk cargo test --manifest-path src-tauri/Cargo.toml display` : attendu, 7 tests PASS.
  - `cargo run --manifest-path src-tauri/Cargo.toml --example list_modes` : attendu, l'écran principal est listé avec son mode natif et la fréquence correcte.
- [ ] **Step 8 : Commit**
  `rtk git add -A && rtk git commit -m "feat(display): display mode model, mode picking and Win32 backend"`

---

### Task 3 : Spike `CDS_FULLSCREEN` + gardien si nécessaire

**Files :**
- Create : `src-tauri/examples/cds_spike.rs`, `docs/superpowers/notes/2026-11-16-cds-fullscreen-spike.md`
- Create (conditionnel) : `src-tauri/src/win/guardian.rs`
- Modify (conditionnel) : `src-tauri/src/main.rs`

**Interfaces :**
- Consumes : `Win32Display`, `restore_all()`, `pick_mode`, `default_resolution` (Task 2).
- Produces (conditionnel) :
  - `pub fn win::guardian::run(parent_pid: u32)` : bloque jusqu'à la mort du parent, puis appelle `restore_all()` ;
  - `pub fn win::guardian::spawn_for_current_process() -> std::io::Result<()>`.

- [ ] **Step 1 : Écrire `cds_spike.rs`**
  - Écran principal → `default_resolution` → `pick_mode` à la fréquence native → `apply_temporary`.
  - Affiche `PID=<pid>, lance : taskkill /F /PID <pid>`, puis attend 120 s.
- [ ] **Step 2 : Exécuter le spike et noter le résultat dans la note**
  - Lancer l'exemple, puis `taskkill /F /PID <pid>` depuis un autre terminal.
  - Noter (oui ou non) :
    - (a) l'écran revient-il en natif dans les 2 s ?
    - (b) pendant l'application, l'image était-elle étirée sans bandes noires alors que la mise à l'échelle GPU est en réglage par défaut ? Cela indique si `DMDFO_STRETCH` a un effet.
  - Noter aussi le GPU et l'écran utilisés.
- [ ] **Step 3 (seulement si (a) = non) : gardien**
  - `main.rs` : si `args[1] == "--guardian"`, appeler `win::guardian::run(args[2].parse())` puis `return`, **avant** tout code Tauri.
  - `spawn_for_current_process` relance `current_exe()` avec `--guardian <pid>` et `CREATE_NO_WINDOW`.
  - Le gardien attend avec `OpenProcess(SYNCHRONIZE)` + `WaitForSingleObject(INFINITE)`.
  - L'appel à `spawn_for_current_process` sera branché en Task 9.
  - Vérifier : ajouter l'appel en tête du spike, refaire le Step 2. Attendu : retour natif en moins de 2 s.
- [ ] **Step 4 : Commit**
  `rtk git add -A && rtk git commit -m "chore: CDS_FULLSCREEN kill spike (+ guardian if needed)"`

---

### Task 4 : Configuration et stockage (`config.rs`, `store.rs`)

**Files :**
- Create : `src-tauri/src/config.rs`, `src-tauri/src/store.rs`

**Interfaces :**
- Consumes : `DisplayMode`, `default_resolution` (Task 2) ; `GameRule` (défini en Task 5, voir ci-dessous).
- Produces :
```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GameEntry { pub id: String, pub name: String, pub exe_name: String, pub exe_path: String,
                       pub width: u32, pub height: u32, pub enabled: bool, pub keep_stretched: bool }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings { pub autostart: bool }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config { pub version: u32, pub games: Vec<GameEntry>, pub settings: Settings } // Default: version 1, vide

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConfigError {
    #[error("Ce jeu est déjà dans la liste")] AlreadyExists(String),
    #[error("Jeu introuvable")] NotFound(String),
}

impl GameEntry { pub fn from_exe_path(path: &str, modes: &[DisplayMode], native: DisplayMode) -> GameEntry; }
impl Config {
    pub fn add_game(&mut self, game: GameEntry) -> Result<(), ConfigError>;
    pub fn update_game(&mut self, game: GameEntry) -> Result<(), ConfigError>;
    pub fn remove_game(&mut self, id: &str) -> Result<(), ConfigError>;
    pub fn rules(&self) -> Vec<crate::engine::GameRule>; // jeux enabled uniquement
}
// store.rs
pub fn config_path() -> std::path::PathBuf;               // %APPDATA%\REZ\config.json
pub fn load(path: &std::path::Path) -> Config;
pub fn save(path: &std::path::Path, config: &Config) -> std::io::Result<()>;
```
Champs de `GameEntry::from_exe_path` :
- `id` = nom de l'exe en minuscules ;
- `name` = nom du fichier sans extension ;
- `width` et `height` via `default_resolution` ;
- `enabled = true`, `keep_stretched = false`.

Pour compiler cette task avant la Task 5, créer `engine.rs` avec seulement la struct `GameRule` (définie en Task 5).

- [ ] **Step 1 : Écrire les tests**
```rust
// config.rs
#[test] fn from_exe_path_builds_entry() {
    let modes = [m(1920,1080,144), m(1440,1080,144)];
    let g = GameEntry::from_exe_path(r"C:\Games\Rust\RustClient.exe", &modes, m(1920,1080,144));
    assert_eq!((g.id.as_str(), g.name.as_str(), g.exe_name.as_str()), ("rustclient.exe", "RustClient", "RustClient.exe"));
    assert_eq!((g.width, g.height, g.enabled, g.keep_stretched), (1440, 1080, true, false));
}
#[test] fn add_same_exe_twice_fails_case_insensitive() {
    let mut c = Config::default();
    c.add_game(GameEntry::from_exe_path(r"C:\A\cs2.exe", &[], m(1920,1080,60))).unwrap();
    let dup = GameEntry::from_exe_path(r"D:\B\CS2.EXE", &[], m(1920,1080,60));
    assert_eq!(c.add_game(dup), Err(ConfigError::AlreadyExists("cs2.exe".into())));
}
#[test] fn rules_exclude_disabled_games() { /* 2 jeux, 1 disabled → rules().len() == 1, exe_name_lower == "cs2.exe" */ }
#[test] fn update_and_remove_unknown_id_fail() { /* → Err(NotFound("x")) pour les deux */ }
// store.rs
#[test] fn missing_file_returns_default() { /* load(tmp/"config.json") == Config::default() */ }
#[test] fn roundtrip() { /* save puis load == original ; aucun fichier "*.tmp" restant */ }
#[test] fn corrupt_file_is_backed_up_and_default_returned() {
    /* écrire "{not json" → load == default ; un fichier dont le nom commence par "config.json.corrupt-" existe */
}
#[test] fn missing_fields_get_defaults() {
    /* écrire {"games":[{"id":"cs2.exe","exeName":"cs2.exe","width":1280,"height":960}]}
       → version == 1, games[0].enabled == false (défaut bool), keep_stretched == false, settings.autostart == false */
}
```
`GameEntry` dérive `Default`, donc `enabled = false` par défaut. Une entrée sans `enabled` dans le fichier reste désactivée, ce qui est le choix sûr. Le test le fixe.

- [ ] **Step 2 : Lancer les tests et vérifier qu'ils échouent.** Commande : `rtk cargo test --manifest-path src-tauri/Cargo.toml config store`.
- [ ] **Step 3 : Implémenter**
  - `save` écrit `config.json.tmp` puis fait un `std::fs::rename`, qui remplace la cible sous Windows.
  - `load` sauvegarde un fichier corrompu en `config.json.corrupt-<secondes unix>`.
  - `config_path` crée le dossier si besoin.
- [ ] **Step 4 : Vérifier.** Même commande. Attendu : 8 tests PASS.
- [ ] **Step 5 : Commit**
  `rtk git add -A && rtk git commit -m "feat(config): game entries, rules, atomic JSON store"`

---

### Task 5 : Machine d'état (`engine.rs`)

**Files :**
- Create/complete : `src-tauri/src/engine.rs`

**Interfaces :**
- Consumes : `DisplayMode`, `pick_mode` (Task 2).
- Produces :
```rust
pub const LEAVE_DEBOUNCE: Duration = Duration::from_millis(250);
#[derive(Clone, Debug, PartialEq)]
pub struct GameRule { pub id: String, pub exe_name_lower: String, pub width: u32, pub height: u32, pub keep_stretched: bool }

pub enum Event {
    Foreground { pid: u32, exe_name: String, monitor: String, modes: Vec<DisplayMode>, native: DisplayMode },
    ProcessExited { pid: u32 },
    DisplayChanged { monitor: String, current: DisplayMode },
    RestoreRequested,
    Shutdown,
    RulesChanged(Vec<GameRule>),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    SaveDesktopLayout { monitor: String },
    Apply { monitor: String, mode: DisplayMode },
    Restore { monitor: String },
    RestoreDesktopLayout { monitor: String },
    WatchProcess { pid: u32 },
    Status(StatusEvent),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StatusEvent {
    Idle,
    Active { game_id: String, width: u32, height: u32 },
    ModeUnavailable { game_id: String, width: u32, height: u32 },
    Error { message: String },
}
impl Engine {
    pub fn new(rules: Vec<GameRule>) -> Self;
    pub fn handle(&mut self, event: Event, now: Instant) -> Vec<Action>;
    pub fn tick(&mut self, now: Instant) -> Vec<Action>;
    pub fn next_deadline(&self) -> Option<Instant>;
    pub fn active_monitor(&self) -> Option<&str>;
    pub fn status(&self) -> StatusEvent;
}
```
**Ordres d'actions fixés** (les tests les vérifient à l'identique) :
- entrée depuis `Idle` : `SaveDesktopLayout, Apply, WatchProcess, Status(Active)` ;
- sortie : `Restore, RestoreDesktopLayout, Status(Idle)`.

- [ ] **Step 1 : Écrire les tests.** Fixtures communes : `D1 = r"\\.\DISPLAY1"`, `D2 = r"\\.\DISPLAY2"`, natif `m(1920,1080,240)`, modes `[m(1920,1080,240), m(1440,1080,240), m(1280,960,240)]`. Règles : `valorant` (`valorant-win64-shipping.exe`, 1440x1080) et `cs2` (`cs2.exe`, 1280x960). Helper `fg(pid, exe, monitor)`.
```rust
#[test] fn enter_game_applies_mode_with_native_refresh() {
    let a = e.handle(fg(10, "valorant-win64-shipping.exe", D1), t0);
    assert_eq!(a, vec![SaveDesktopLayout{monitor:D1.into()}, Apply{monitor:D1.into(), mode:m(1440,1080,240)},
                       WatchProcess{pid:10}, Status(StatusEvent::Active{game_id:"valorant".into(), width:1440, height:1080})]);
}
#[test] fn exe_match_is_case_insensitive() { /* fg(10,"VALORANT-Win64-Shipping.exe",D1) → contient Apply */ }
#[test] fn leave_restores_after_debounce() {
    /* entrée à t0 ; fg(99,"chrome.exe",D1) à t1 → vec![] ; tick(t1+249ms) → vec![] ;
       tick(t1+250ms) → [Restore{D1}, RestoreDesktopLayout{D1}, Status(Idle)] */
}
#[test] fn quick_return_cancels_restore() { /* sortie à t1, retour au jeu à t1+100ms → vec![] ; tick(t1+300ms) → vec![] */ }
#[test] fn switch_game_to_game_same_monitor_applies_directly() {
    /* valorant actif, fg(20,"cs2.exe",D1) → [Apply{D1,m(1280,960,240)}, WatchProcess{20}, Status(Active{cs2,1280,960})] */
}
#[test] fn switch_across_monitors_restores_first() {
    /* valorant sur D1, fg(20,"cs2.exe",D2) → [Restore{D1}, RestoreDesktopLayout{D1}, SaveDesktopLayout{D2}, Apply{D2,..}, WatchProcess{20}, Status(Active{cs2..})] */
}
#[test] fn mode_unavailable_reports_and_stays_idle() {
    /* modes sans 1440x1080 → [Status(ModeUnavailable{valorant,1440,1080})] ; active_monitor() == None */
}
#[test] fn keep_stretched_ignores_leave_and_restores_on_exit() {
    /* règle keep_stretched ; fg chrome → [] ; tick(+1s) → [] ; ProcessExited{10} → [Restore, RestoreDesktopLayout, Status(Idle)] */
}
#[test] fn process_exit_restores_immediately() { /* jeu normal actif, ProcessExited{10} → sortie complète, sans attendre le délai */ }
#[test] fn exit_of_other_pid_is_ignored() { /* ProcessExited{777} → [] */ }
#[test] fn display_changed_while_active_reapplies() {
    /* DisplayChanged{D1, m(1920,1080,240)} → [Apply{D1,m(1440,1080,240)}] ; DisplayChanged{D1, m(1440,1080,240)} → [] */
}
#[test] fn display_changed_while_idle_or_leaving_is_ignored() { /* Idle → [] ; pendant le délai de sortie → [] */ }
#[test] fn restore_requested_goes_idle() { /* actif → [Restore, RestoreDesktopLayout, Status(Idle)] ; Idle → [] */ }
#[test] fn shutdown_restores_when_active() { /* actif → sortie complète ; Idle → [] */ }
#[test] fn rules_changed_removing_active_game_restores() { /* RulesChanged(vec![cs2]) pendant valorant actif → sortie complète */ }
#[test] fn rules_changed_new_resolution_reapplies() { /* valorant passe en 1280x960 pendant qu'il est actif → [Apply{D1,m(1280,960,240)}, Status(Active{valorant,1280,960})] */ }
```
- [ ] **Step 2 : Lancer les tests et vérifier qu'ils échouent.** Commande : `rtk cargo test --manifest-path src-tauri/Cargo.toml engine`.
- [ ] **Step 3 : Implémenter `Engine`**
  - État : `Idle`, ou `Active { game_id, pid, monitor, mode, keep_stretched, leave_deadline: Option<Instant> }`.
  - `next_deadline` = `leave_deadline`.
  - L'état `Active` garde aussi les `modes` et le `native` reçus dans le `Foreground`. `RulesChanged` peut ainsi recalculer le mode sans nouvel événement.
- [ ] **Step 4 : Vérifier.** Même commande. Attendu : 16 tests PASS.
- [ ] **Step 5 : Commit**
  `rtk git add -A && rtk git commit -m "feat(engine): pure foreground/display state machine"`

---

### Task 6 : Événements Win32 (`win/events.rs`)

**Files :**
- Create : `src-tauri/src/win/events.rs`, `src-tauri/examples/watch_events.rs`

**Interfaces :**
- Consumes : `Input` (défini en Task 7, à déclarer dès maintenant dans `runtime.rs` avec cette forme exacte) :
```rust
pub enum Input {
    Foreground { pid: u32, exe_name: String, monitor: String },
    ProcessExited { pid: u32 },
    DisplayMaybeChanged,
    RestoreRequested,
    RulesChanged(Vec<GameRule>),
    Shutdown { ack: std::sync::mpsc::Sender<()> },
}
```
- Produces :
  - `pub fn start(tx: Sender<Input>) -> std::io::Result<()>` : lance un thread et rend la main une fois le hook installé ;
  - `pub fn watch_process(pid: u32, tx: Sender<Input>)`.

- [ ] **Step 1 : Implémenter `start`.** Un thread unique, avec sa propre boucle de messages, porte :
  - **Le hook** : `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, cb, 0, 0, WINEVENT_OUTOFCONTEXT)`.
    - Le callback récupère hwnd → `GetWindowThreadProcessId`, puis `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW`, et ne garde que le nom du fichier.
    - Si la lecture échoue : `exe_name = ""` et `log::warn!`.
    - Écran : `MonitorFromWindow(MONITOR_DEFAULTTONEAREST)` → `GetMonitorInfoW(MONITORINFOEXW).szDevice`.
    - Le `Sender` est rangé dans un `static OnceLock<Mutex<Sender<Input>>>`.
  - **Une fenêtre top-level cachée** (classe `REZ_EVENTS`, jamais affichée). Les fenêtres message-only ne reçoivent pas les broadcasts, d'où une vraie fenêtre.
    - `WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)`.
    - `WM_DISPLAYCHANGE` → `DisplayMaybeChanged`.
    - `WM_WTSSESSION_CHANGE` avec `WTS_SESSION_UNLOCK` → `DisplayMaybeChanged`.
    - `WM_ENDSESSION` avec wParam TRUE → appel **synchrone** de `win::display::restore_all()`.
- [ ] **Step 2 : Implémenter `watch_process`.** Un thread par pid : `OpenProcess(SYNCHRONIZE)` + `WaitForSingleObject(INFINITE)` → `ProcessExited`. Si l'ouverture échoue, on log et on n'envoie rien.
- [ ] **Step 3 : `examples/watch_events.rs`.** Appelle `start`, puis affiche chaque `Input` reçu.
- [ ] **Step 4 : Vérifier**
  - Lancer `cargo run --manifest-path src-tauri/Cargo.toml --example watch_events`.
  - Alterner entre le Bloc-notes et un navigateur : attendu, `Foreground { exe_name: "notepad.exe", monitor: "\\.\DISPLAY1", .. }` à chaque bascule.
  - Win+L puis déverrouiller : attendu, `DisplayMaybeChanged`.
- [ ] **Step 5 : Commit**
  `rtk git add -A && rtk git commit -m "feat(win): foreground hook, display/session events, process watch"`

---

### Task 7 : Runtime (`runtime.rs`)

**Files :**
- Create/complete : `src-tauri/src/runtime.rs`

**Interfaces :**
- Consumes : `Engine`, `Event`, `Action`, `StatusEvent`, `GameRule` (Task 5) ; `DisplayApi` (Task 2) ; `Input` (forme fixée en Task 6).
- Produces :
```rust
pub trait LayoutApi: Send { fn capture(&mut self, monitor: &str); fn restore(&mut self, monitor: &str); }
pub struct Runtime { /* engine, display, layout, watch_process, on_status, modes cache */ }
impl Runtime {
    pub fn new(rules: Vec<GameRule>, display: Arc<dyn DisplayApi>, layout: Box<dyn LayoutApi>,
               watch_process: Box<dyn Fn(u32) + Send>, on_status: Box<dyn Fn(StatusEvent) + Send>) -> Self;
    pub fn handle_input(&mut self, input: Input, now: Instant);
    pub fn tick(&mut self, now: Instant);
    pub fn spawn(self, rx: Receiver<Input>) -> JoinHandle<()>; // thread : recv_timeout(next_deadline - now), puis handle_input / tick
    // Le canal (tx, rx) est créé par l'appelant : tx est partagé avec watch_process et win::events::start.
}
```
**Règles du runtime :**
- **`Foreground`** :
  - `exe_name` vide → l'événement est transmis à l'engine comme un non-jeu (vers `chrome`, l'effet est identique) ;
  - sinon, lire `list_modes` et `registry_mode` du moniteur (cache par moniteur pour la session), puis construire `Event::Foreground`.
- **`DisplayMaybeChanged`** : si `engine.active_monitor()` est `Some(m)`, construire `DisplayChanged { m, current_mode(m) }`.
- **Erreur d'`apply_temporary` ou `restore`** : `on_status(Error { message })` et log. On continue avec les actions suivantes.
- **`Status`** → `on_status`.
- **`Shutdown { ack }`** : traité, puis `ack.send(())`, puis fin du thread.

- [ ] **Step 1 : Écrire les tests**
  - `MockDisplay` :
    - enregistre les appels dans `Arc<Mutex<Vec<String>>>`, au format `"apply \\.\DISPLAY1 1440x1080@240"` / `"restore \\.\DISPLAY1"` ;
    - `list_modes` renvoie `[1920x1080@240, 1440x1080@240, 1280x960@240]` ;
    - `registry_mode` renvoie `1920x1080@240` ;
    - `current_mode` est réglable (par défaut, le dernier mode appliqué) ;
    - option `fail_apply`.
  - `MockLayout` : enregistre `"capture …"` / `"restore …"`.
```rust
#[test] fn runtime_applies_and_restores_via_apis() {
    /* Input::Foreground{10,"cs2.exe",D1} → display == ["apply \\.\DISPLAY1 1280x960@240"], layout == ["capture \\.\DISPLAY1"], watch == [10] ;
       Foreground chrome à t1, tick(t1+250ms) → display se termine par "restore \\.\DISPLAY1", layout par "restore \\.\DISPLAY1" ;
       statuts reçus : [Active{cs2,1280,960}, Idle] */
}
#[test] fn runtime_ignores_unreadable_exe() { /* Foreground{10,"",D1} → aucun appel display, aucun statut */ }
#[test] fn runtime_reports_apply_error() { /* fail_apply → statuts contiennent Error{..} ; Shutdown suivant → "restore" appelé quand même */ }
#[test] fn runtime_reapplies_on_display_change() { /* actif, mock current_mode → natif, DisplayMaybeChanged → second "apply" */ }
#[test] fn runtime_shutdown_acks() { /* Shutdown{ack} sur runtime actif → "restore" appelé puis ack reçu */ }
```
- [ ] **Step 2 : Lancer les tests et vérifier qu'ils échouent.** Commande : `rtk cargo test --manifest-path src-tauri/Cargo.toml runtime`.
- [ ] **Step 3 : Implémenter `Runtime`.** `spawn` boucle sur `rx.recv_timeout(...)`.
- [ ] **Step 4 : Vérifier.** Même commande. Attendu : 5 tests PASS, et l'ensemble de `cargo test` reste vert.
- [ ] **Step 5 : Commit**
  `rtk git add -A && rtk git commit -m "feat(runtime): drive engine actions through display/layout APIs"`

---

### Task 8 : Disposition du bureau (`win/desktop_layout.rs`)

**Files :**
- Create : `src-tauri/src/win/desktop_layout.rs`

**Interfaces :**
- Consumes : `LayoutApi` (Task 7).
- Produces : `pub struct Win32Layout` (`Win32Layout::new()`, impl `LayoutApi`).

- [ ] **Step 1 : Implémenter**
  - **COM** : `CoInitializeEx(COINIT_APARTMENTTHREADED)` au début de chaque `capture` et `restore` (`S_FALSE` est acceptable).
  - **Icônes, capture** :
    - `IShellWindows::FindWindowSW(CSIDL_DESKTOP, …, SWC_DESKTOP, SWFO_NEEDDISPATCH)` ;
    - puis `IServiceProvider::QueryService(SID_STopLevelBrowser)` → `IShellBrowser::QueryActiveShellView` → `IFolderView` ;
    - parcourir `Items(SVGIO_ALLVIEW)` et `GetItemPosition`, en stockant les PIDL clonés.
  - **Icônes, restauration** : `SelectAndPositionItems(…, SVSI_POSITIONITEM)`. Les icônes sont capturées quel que soit l'écran.
  - **Fenêtres** : `EnumWindows`, en gardant les fenêtres visibles, non minimisées, top-level, dont `MonitorFromWindow` correspond au device. Capture avec `GetWindowPlacement`, restauration avec `SetWindowPlacement`.
  - **Erreurs** : chaque erreur donne un `log::warn!`, jamais une panique ni une remontée. Le basculement ne doit jamais être bloqué.
- [ ] **Step 2 : Vérifier.** Pour ce test manuel, créer un exemple jetable, non commité :
  - icônes réparties sur tout le bureau, Bloc-notes à droite de l'écran ;
  - `capture` → `apply_temporary(1280x960)` → attendre 3 s → `restore` (display) → `restore` (layout) ;
  - attendu : icônes et Bloc-notes à leur place d'origine.
- [ ] **Step 3 : Commit**
  `rtk git add -A && rtk git commit -m "feat(win): save/restore desktop icons and window placements"`

---

### Task 9 : Intégration Tauri (`lib.rs`, `commands.rs`, `tray.rs`)

**Files :**
- Create : `src-tauri/src/commands.rs`, `src-tauri/src/tray.rs`
- Modify : `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json`

**Interfaces :**
- Consumes : tout ce qui précède.
- Produces : commandes IPC et événement, utilisés par l'UI en Task 10.
```rust
#[serde(rename_all = "camelCase")] pub struct Resolution { pub width: u32, pub height: u32 }
#[serde(rename_all = "camelCase")] pub struct DisplayInfo { pub device: String, pub label: String, pub primary: bool,
                                                             pub native: DisplayMode, pub resolutions: Vec<Resolution> }
#[tauri::command] fn get_config(state) -> Config;
#[tauri::command] fn list_displays(state) -> Result<Vec<DisplayInfo>, String>;
#[tauri::command] fn add_game(state, exe_path: String) -> Result<GameEntry, String>;  // défauts depuis l'écran principal
#[tauri::command] fn update_game(state, game: GameEntry) -> Result<(), String>;
#[tauri::command] fn remove_game(state, id: String) -> Result<(), String>;
#[tauri::command] fn restore_native(state);
#[tauri::command] fn get_status(state) -> StatusEvent;
#[tauri::command] fn set_autostart(app, state, enabled: bool) -> Result<(), String>;
// événement émis : "rez://status", payload StatusEvent
```
`AppState { config: Mutex<Config>, config_path: PathBuf, input: Sender<Input>, status: Mutex<StatusEvent>, display: Arc<Win32Display> }`. Chaque mutation de config fait `store::save` puis `input.send(RulesChanged(config.rules()))`.

- [ ] **Step 1 : Plugins Tauri 2** : `tauri-plugin-single-instance`, `tauri-plugin-global-shortcut`, `tauri-plugin-autostart`, `tauri-plugin-dialog`, `tauri-plugin-log`. Feature `tray-icon` sur `tauri`. Capability : `core:default`, `dialog:allow-open`.
- [ ] **Step 2 : Câbler `lib.rs::run()`**
  - **Panic hook** : chaîne le hook précédent et appelle `restore_all()`.
  - **Gardien** : si la Task 3 l'a créé, appeler `guardian::spawn_for_current_process()`.
  - **Démarrage** :
    - `store::load` ;
    - `let (tx, rx) = channel()` ;
    - `Runtime::new(..., watch_process = move |pid| win::events::watch_process(pid, tx.clone()), ...).spawn(rx)` ;
    - `win::events::start(tx.clone())` ;
    - `tx` est rangé dans `AppState.input`.
  - **Statut** : `on_status` met à jour `AppState.status`, émet `rez://status` et rafraîchit l'infobulle de l'icône.
  - **Raccourci** Ctrl+Alt+Shift+R → `Input::RestoreRequested`.
  - **Autostart** : `MacosLauncher::LaunchAgent`, args `["--minimized"]`.
  - **Single-instance** : une seconde instance appelle `show_main_window`.
- [ ] **Step 3 : Fenêtre et sortie**
  - `pub fn show_main_window(app: &AppHandle)` : donne le focus à la fenêtre `main` si elle existe, sinon la construit avec `WebviewWindowBuilder` (titre `REZ`, 960x640, min 820x560).
  - Au lancement sans `--minimized`, appeler `show_main_window`.
  - `RunEvent::ExitRequested { code: None }` → `api.prevent_exit()`. Fermer la fenêtre détruit la WebView, l'app reste près de l'horloge.
  - `RunEvent::Exit` → `Shutdown { ack }`, puis `ack_rx.recv_timeout(2 s)`.
- [ ] **Step 4 : `tray.rs`**
  - Infobulle `REZ — inactif` ou `REZ — <nom> <w>×<h>`.
  - Clic gauche → `show_main_window`.
  - Menu : `open` « Ouvrir REZ », `restore` « Revenir en résolution native » (→ `RestoreRequested`), `quit` « Quitter » (→ `app.exit(0)`).
- [ ] **Step 5 : Vérifier**
  - `rtk cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` : attendu, 0 warning.
  - Lancer `npm run tauri dev`, puis depuis la console de la WebView : `await __TAURI_INTERNALS__.invoke("add_game", { exePath: "C:\\Windows\\System32\\notepad.exe" })`.
  - Ouvrir le Bloc-notes : attendu, passage en 1440x1080 (ou défaut 4:3).
  - Alt-tab : attendu, retour natif.
  - Menu de l'icône « Quitter » pendant que c'est étiré : attendu, retour natif avant la fermeture.
- [ ] **Step 6 : Commit**
  `rtk git add -A && rtk git commit -m "feat(app): Tauri wiring, commands, tray, emergency hotkey, autostart"`

---

### Task 10 : Interface (Jeux + Réglages)

**Files :**
- Create : `src/i18n/index.ts`, `src/i18n/fr.json`, `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/format.ts`, `src/lib/format.test.ts`, `src/pages/Games.tsx`, `src/pages/Settings.tsx`, `src/components/GameCard.tsx`, `src/components/StatusBar.tsx`
- Modify : `src/App.tsx`, `src/main.tsx`, `src/index.css`, `vite.config.ts`, `package.json`

**Interfaces :**
- Consumes : commandes et événement de la Task 9. Les types TS de `types.ts` reprennent les champs camelCase : `GameEntry`, `Config`, `DisplayInfo`, `DisplayMode`, `StatusEvent` (union discriminée sur `kind`).
- Produces :
  - `aspectLabel(width: number, height: number): string`, par PGCD ;
  - `formatResolution(width: number, height: number): string` → `"1440 × 1080"`.

- [ ] **Step 1 : Dépendances**
  - `npm i react-i18next i18next @tauri-apps/plugin-dialog`.
  - `npm i -D tailwindcss @tailwindcss/vite vitest`.
  - Plugin Tailwind dans `vite.config.ts`, `@import "tailwindcss";` dans `index.css`.
  - Script `"test": "vitest run"`.
- [ ] **Step 2 : Écrire les tests (`format.test.ts`)**
```ts
expect(aspectLabel(1440, 1080)).toBe("4:3");
expect(aspectLabel(1280, 1024)).toBe("5:4");
expect(aspectLabel(1920, 1080)).toBe("16:9");
expect(aspectLabel(1680, 1050)).toBe("16:10");
expect(formatResolution(1440, 1080)).toBe("1440 × 1080");
```
- [ ] **Step 3 : Lancer les tests et vérifier qu'ils échouent.** Commande : `rtk npx vitest run`.
- [ ] **Step 4 : Implémenter `format.ts`**, puis l'UI. Thème sombre, sobre, gaming. Tous les textes viennent de `fr.json`.
  - **`App.tsx`** : onglets « Jeux » / « Réglages », `StatusBar` en haut.
  - **`StatusBar`** :
    - lit `get_status` au montage, puis écoute `rez://status` ;
    - affiche `Idle`, `Active` (nom du jeu, `formatResolution`), `ModeUnavailable` (« Résolution … indisponible sur cet écran ») ou `Error` ;
    - bouton « Revenir en natif » → `restore_native`.
  - **`Games`** :
    - liste de `GameCard` et bouton « Ajouter un jeu » ;
    - l'ajout ouvre le sélecteur de fichier avec un filtre `exe`, puis appelle `add_game` ;
    - les erreurs (« déjà dans la liste ») s'affichent sous le bouton.
  - **`GameCard`** :
    - nom et exe ;
    - liste déroulante des résolutions de l'écran principal, au format `"1440 × 1080 (4:3)"` ;
    - interrupteurs « Activé » et « Rester étiré pendant alt-tab » ;
    - « Retirer » avec confirmation ;
    - chaque changement appelle `update_game`.
  - **`Settings`** :
    - interrupteur « Lancer REZ au démarrage de Windows » → `set_autostart` ;
    - texte « Ctrl+Alt+Shift+R : retour à la résolution native ».
- [ ] **Step 5 : Vérifier**
  - `rtk npx vitest run` : attendu, 5 assertions PASS.
  - `rtk tsc --noEmit` : attendu, 0 erreur.
  - `npm run tauri dev` : ajouter le Bloc-notes via l'UI, changer sa résolution, puis le mettre au premier plan. Attendu : la `StatusBar` passe en « Active ».
- [ ] **Step 6 : Commit**
  `rtk git add -A && rtk git commit -m "feat(ui): games list, settings, live status"`

---

### Task 11 : Vérification de bout en bout de la phase 1

**Files :**
- Create : `docs/superpowers/notes/2026-11-16-phase1-verification.md` (résultats cochés)

- [ ] **Step 1 : Tests automatiques.**
  - `rtk cargo test --manifest-path src-tauri/Cargo.toml` : tout est vert.
  - `rtk cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` : 0 warning.
  - `rtk npx vitest run` : vert.
- [ ] **Step 2 : Scénarios manuels** (`npm run tauri dev`, Bloc-notes configuré en 1280x960). Noter OK ou KO pour chacun :
  1. Bloc-notes au premier plan → 1280x960 à la fréquence native. Vérifier dans Paramètres > Affichage avancé.
  2. Alt-tab vers le navigateur → natif. Retour en moins de 250 ms → pas de clignotement.
  3. `keepStretched` actif → reste en 1280x960 en alt-tab, revient en natif à la fermeture du Bloc-notes.
  4. `taskkill /F` de REZ pendant que c'est étiré → natif en moins de 2 s (par Windows ou par le gardien).
  5. Icônes du bureau et fenêtres à leur place après un aller-retour.
  6. Win+L pendant que c'est étiré, puis déverrouiller avec le Bloc-notes au premier plan → mode réappliqué.
  7. Ctrl+Alt+Shift+R → natif.
  8. Fermer la fenêtre → REZ reste près de l'horloge. Une seconde instance ouvre la fenêtre existante.
  9. Deux écrans, si disponibles : Bloc-notes sur l'écran 2 → seul l'écran 2 change.
  10. Rust et ROB RUN en borderless avec la mise à l'échelle GPU en « plein écran » → image étirée sans bandes noires, retour natif en alt-tab.
- [ ] **Step 3 : Build.** `npm run tauri build` → l'installateur NSIS est produit dans `src-tauri/target/release/bundle/nsis/`. L'installer, le lancer et refaire le scénario 1.
- [ ] **Step 4 : Commit**
  `rtk git add -A && rtk git commit -m "docs: phase 1 verification results"`
