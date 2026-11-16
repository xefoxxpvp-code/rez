# Spike CDS_FULLSCREEN — 2026-11-16

Setup: NVIDIA GeForce RTX 5060 Ti (driver 32.0.16.2101). DISPLAY1 = Acer KG241Y P3, DisplayPort, 1920x1080@180 (primary). DISPLAY2 = Dell SE2417HGX, HDMI, 1920x1080@75.

| Run | Target | Mode | DMDFO_STRETCH | End | Result |
|---|---|---|---|---|---|
| 1 | Acer | 1440x1080@180 | yes (then fallback) | force-kill | **Acer detached from the desktop topology** (still connected per WMI); Dell became primary. Topology later restored. |
| 2 | Dell (primary at that time) | 1440x1080@75 | yes | force-kill | Back to 1920x1080@75 within 1.5 s |
| E1 | Acer | 1440x1080@180 | no | normal restore | OK, apply returned in 208 ms, topology intact |
| E2 | Acer | 1440x1080@180 | yes | normal restore | OK, topology intact |
| E3 | Acer | 1440x1080@180 | yes | force-kill | Native within ~1.3 s, topology intact |

Conclusions:
- (a) Windows restores a CDS_FULLSCREEN mode when the owning process dies: **yes** (runs 2 and E3). No guardian process needed.
- (b) Visual effect of DMDFO_STRETCH: not observed yet (user did not watch run 1).
- Run 1 topology loss is **not reproducible** in 3 controlled runs. Only differences: first mode switch of the session on that monitor. Hypothesis (unconfirmed): long first resync on the DisplayPort monitor read as a hot-unplug.
- Mitigation needed regardless: detect a monitor disappearing after a switch and put the saved display configuration back.

Tools: `src-tauri/examples/cds_spike.rs`, `src-tauri/examples/cds_probe.rs` (timeline of the topology every 250 ms).
