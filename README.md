# REZ

Per-game stretched resolution (4:3 and more) applied only while the game is in the foreground, plus a cross-game crosshair overlay. Windows 10/11.

Status: early development (phase 1). License: MIT.

## Publier une version

1. Monter la version dans `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et `package.json` (ex. `0.2.1`), committer et pousser sur `main`.
2. `gh release create v0.2.1 --generate-notes`
3. Le workflow *Release* compile, signe et joint l'installateur et `latest.json` à la release. Les utilisateurs voient alors « Mise à jour 0.2.1 disponible » dans REZ.

La clé privée de signature n'est jamais dans le dépôt : elle est dans le secret `TAURI_SIGNING_PRIVATE_KEY`.
