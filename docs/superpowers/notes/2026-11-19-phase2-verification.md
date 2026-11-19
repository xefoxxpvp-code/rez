# Phase 2 — verification (2026-11-19)

## Automated
- cargo test --lib: 120 passed; clippy --all-targets -D warnings: clean
- vitest: 7 passed; tsc --noEmit: clean; vite build: OK
- Signed release build: REZ_0.2.0_x64-setup.exe + .sig (updater artifacts)
- Import vectors: CS2 legacy-v1 x2, cs2-v1 x2, Valorant x2, REZ roundtrip, PNG (corrupt, >256 px)

## Pending (needs the user, not while gaming)
- Crosshair over a game window, hidden on alt-tab, Ctrl+Alt+Shift+X toggle
- Square crosshair while stretched (undistort on/off)
- In-app update end to end (needs the GitHub repo + secrets, then a v0.2.1 tag)
