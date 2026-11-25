# Verenu Fidelity, Hotkey & Residency Customizations

**Status:** Patch Series & Source Diff (`v0.20.0-custom`)

Focused source customizations built on top of upstream [Verenu](https://github.com/Verenu/Verenu) (`v0.20.0`, commit `5e712862d2f5a73b0ed36fdef3d9bbd1b125c1cb`). Includes deterministic wording-preservation cleanup validation, single-press hotkey gesture refinements, and conditional local LLM/STT model residency management.

## Upstream base & license

- **Upstream repository:** `https://github.com/Verenu/Verenu.git`
- **Base commit:** `5e712862d2f5a73b0ed36fdef3d9bbd1b125c1cb`
- **License:** MIT - see [LICENSE](LICENSE), retaining upstream copyright (`Copyright (c) 2026 MONKE2525E`).

## What is included

- `patches/verenu-5e71286-customizations.patch` - Complete unified diff against `5e712862d2f5a73b0ed36fdef3d9bbd1b125c1cb`.
- Scoped patch slices under `patches/`:
  - `01-fidelity-cleanup.patch` - Strict wording-preservation validator (`src-tauri/src/pipeline/fidelity.rs`, `fidelity_tests.rs`, `stages_cleanup.rs`, `cache.rs`, `docs/CLEANUP_LEVELS.md`).
  - `02-toggle-hotkey-and-session.patch` - Single-press toggle gesture and hotkey state fixes (`gesture.rs`, `win.rs`, `app_hotkey.rs`, `devSession.ts`).
  - `03-conditional-model-residency.patch` - Memory-aware local LLM/STT residency policy (`model_residency.rs`, `residency_policy.rs`, `memory.rs`, `local_llm/`, `local_stt/`).
  - `04-lib-wiring-and-changelog.patch` - Tauri wiring (`src-tauri/src/lib.rs`) and changelog notes.
- `files/` - Full copies of all 24 modified and newly added files for direct browsing.

Private user prompts, recordings, and local settings databases are excluded.

## Applying the patch

```powershell
git clone https://github.com/Verenu/Verenu.git
cd Verenu
git checkout 5e712862d2f5a73b0ed36fdef3d9bbd1b125c1cb
git apply ..\patches\verenu-5e71286-customizations.patch
```

## Validation (on base `5e71286` + customizations)

- **Rust unit & pipeline tests (`cargo test`):** 988 passed, 0 failed, 4 ignored.
- **Frontend unit tests (`vitest run --dir src`):** 38 files, 321 tests passed.
