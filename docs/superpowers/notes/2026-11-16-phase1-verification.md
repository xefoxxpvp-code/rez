# Phase 1 — verification (2026-11-16)

Machine: RTX 5060 Ti; DISPLAY1 Acer KG241Y P3 (DP, 1920x1080@180, primary); DISPLAY2 Dell SE2417HGX (HDMI, 1920x1080@75). Test "game": Notepad, 1280x960.

## Automated
- `cargo test` (src-tauri): 39 passed
- `cargo clippy --all-targets -- -D warnings`: clean
- `npx vitest run`: 3 tests (5 assertions) passed; `tsc --noEmit`: clean
- `npm run tauri build`: `REZ_0.1.0_x64-setup.exe` (1.80 MiB)

## Manual scenarios
| # | Scenario | Result |
|---|---|---|
| 1 | Game foreground → mode at native refresh | OK — Dell 1280x960@75 (release and dev builds) |
| 2 | Alt-tab → native | OK. Quick return (<250 ms) covered by engine tests |
| 3 | keepStretched until the game exits | Pending (user) |
| 4 | Hard kill while stretched → native | OK — Windows restores within ~1.5 s (spike runs 2, E3) |
| 5 | Icons/windows back in place | OK — 49/49 icons restored (layout_check) |
| 6 | Win+L then unlock → mode re-applied | Pending (user) |
| 7 | Ctrl+Alt+Shift+R → native | OK (SendKeys, dev and release) |
| 8 | Close window keeps REZ in tray; second instance reuses the first | OK |
| 9 | Two screens: only the game's screen changes | OK — Dell switched, Acer untouched |
| 10 | Rust / ROB RUN borderless stretched without black bars | Pending (user, needs GPU scaling "full screen") |
| — | Tray "Quitter" while stretched → native before exit | Pending (user) |

Memory: release build ~31 MB with window, ~38 MB after closing it (WebView2 helpers not counted).

Known: one topology loss on the Acer during the first spike run, not reproducible; mitigated by the topology guard (Task 7b), which is untested on real hardware.
