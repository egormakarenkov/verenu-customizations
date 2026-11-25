//! Warm only downloaded default models, without running inference or downloads.
use super::{
    memory,
    residency_policy::{has_headroom, WarmupSchedule},
};
use crate::{
    data::store, local_llm::LocalLlmManager, local_stt::LocalTranscriptionManager,
    pipeline::SharedState,
};
use std::sync::atomic::Ordering;
use tauri::AppHandle;

fn idle(state: &SharedState) -> bool {
    state.lock().is_ok_and(|state| state.lifecycle.is_idle())
}

fn headroom(model_mb: u64) -> bool {
    memory::system_memory_status()
        .is_some_and(|mem| has_headroom(mem.available_mb, mem.total_mb, model_mb))
}

pub async fn maintain(
    app: &AppHandle,
    state: &SharedState,
    stt: &LocalTranscriptionManager,
    llm: &LocalLlmManager,
    schedule: &mut WarmupSchedule,
    now_ms: u64,
    healthy: bool,
) -> anyhow::Result<()> {
    let settings = store::settings_snapshot(app).map_err(anyhow::Error::msg)?;
    let cfg = store::load_pipeline_config(&settings);
    if !schedule.ready(
        now_ms,
        healthy,
        cfg.local_model_memory_policy == "keep_loaded",
        idle(state),
    ) {
        return Ok(());
    }
    if stt.shutdown_signal.load(Ordering::Relaxed) || llm.shutdown_signal.load(Ordering::Relaxed) {
        return Ok(());
    }

    let stt_model = store::parse_model_id(&cfg.transcription_default_model)
        .filter(|(provider, _)| provider == store::LOCAL);
    let llm_model = store::parse_model_id(&cfg.cleanup_default_model)
        .filter(|(provider, _)| cfg.cleanup_enabled && provider == store::LOCAL);
    let mut attempted = false;
    if let Some((_, model)) = stt_model {
        let manifest = crate::local_stt::model::manifest_by_id(&model);
        if let Some(manifest) = manifest {
            let root = stt.prepare_models_dir()?;
            if !stt.state().is_loaded && manifest.is_downloaded(&root) && headroom(manifest.size_mb)
            {
                attempted = true;
                schedule.attempted(now_ms);
                let manager = stt.clone();
                let app = app.clone();
                log::info!("local models: background speech preload id={model}");
                tokio::task::spawn_blocking(move || manager.preload_if_idle(&app, &model))
                    .await??;
            }
        }
    }
    // Recheck after speech loading: it may have consumed the spare memory,
    // or dictation/shutdown may have started while the engine initialized.
    if !idle(state) || llm.shutdown_signal.load(Ordering::Relaxed) {
        return Ok(());
    }
    if let Some((_, model)) = llm_model {
        if let Some(manifest) = crate::local_llm::model::manifest_by_id(&model) {
            let root = llm.prepare_models_dir()?;
            let runtime_installed = std::env::var_os("VERENU_LLAMA_SERVER_PATH")
                .map(std::path::PathBuf::from)
                .is_some_and(|path| path.is_file())
                || crate::local_llm::binary::is_runtime_installed(
                    &crate::local_llm::binary::runtime_root(),
                );
            if !llm.state().is_loaded
                && runtime_installed
                && manifest.is_downloaded(&root)
                && headroom(manifest.size_mb)
            {
                let safe = tokio::task::spawn_blocking(memory::detect_resource_pressure)
                    .await?
                    .is_none();
                if safe && idle(state) {
                    if !attempted {
                        schedule.attempted(now_ms);
                    }
                    log::info!("local models: background cleanup preload id={model}");
                    llm.preload_if_idle(app, &model).await?;
                }
            }
        }
    }
    Ok(())
}
