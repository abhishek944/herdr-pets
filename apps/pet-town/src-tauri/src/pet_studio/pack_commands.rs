use super::types::{
    ActivateExtensionRequest, ActivateExtensionResult, DiscardExtensionCandidateRequest,
    PetExtensionCandidateView, PetExtensionCatalogView, SaveExtensionRequest, SavePackRequest,
    UserPackView,
};
use super::{catalog, store};
use super::{with_draft, PetStudioState};
use tauri::{Emitter, Manager};

#[tauri::command]
pub fn save_pet_pack(
    request: SavePackRequest,
    state: tauri::State<'_, PetStudioState>,
    app: tauri::AppHandle,
) -> Result<ActivateExtensionResult, String> {
    let draft_id = request.draft_id.clone();
    let assign_orchestrator = request.assign_to_orchestrator;
    let id = with_draft(&state, &draft_id, |draft| store::save(&request, draft))?;
    let draft = state
        .0
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&draft_id);
    if let Some(draft) = draft {
        let _ = std::fs::remove_dir_all(draft.directory);
    }
    super::clear_orchestrator_pet_cache();
    let snapshot = app
        .state::<crate::preferences::PreferencesStore>()
        .register_pet(id.clone(), assign_orchestrator);
    let _ = crate::street_windows::emit_to_streets(app.clone(), "preferences-applied", &snapshot);
    let _ = app.emit("pet-studio-preferences", &snapshot);
    crate::orchestrator::commands::preferences_changed(&app, None);
    let reload_warning =
        crate::street_windows::emit_to_streets(app.clone(), "user-packs-changed", ())
            .err()
            .map(|_| {
                "The pet was saved, but the village could not reload it automatically.".to_string()
            });
    Ok(ActivateExtensionResult { id, reload_warning })
}

#[tauri::command]
pub fn list_user_pet_packs() -> Result<Vec<UserPackView>, String> {
    super::clear_orchestrator_pet_cache();
    catalog::list()
}

#[tauri::command]
pub fn list_pet_extensions() -> Result<PetExtensionCatalogView, String> {
    super::extension_catalog::list()
}

#[tauri::command]
pub fn save_pet_extension(
    request: SaveExtensionRequest,
    state: tauri::State<'_, PetStudioState>,
) -> Result<PetExtensionCandidateView, String> {
    with_draft(&state, &request.draft_id, |draft| {
        let candidate = super::extension_store::stage(&request, draft)?;
        draft
            .extension_candidates
            .push(super::types::StagedPetExtension {
                base_id: request.base_id.clone(),
                candidate_id: candidate.candidate_id.clone(),
            });
        Ok(candidate)
    })
}

#[tauri::command]
pub fn activate_pet_extension(
    request: ActivateExtensionRequest,
    state: tauri::State<'_, PetStudioState>,
    app: tauri::AppHandle,
) -> Result<ActivateExtensionResult, String> {
    let draft = {
        let mut drafts = state.0.lock().unwrap_or_else(|error| error.into_inner());
        if !drafts.contains_key(&request.draft_id) {
            return Err("This Pet Studio draft is no longer available.".into());
        }
        let id = super::extension_activation::activate(
            &request.base_id,
            &request.candidate_id,
            &request.draft_id,
        )?;
        (id, drafts.remove(&request.draft_id).unwrap())
    };
    let (id, draft) = draft;
    let _ = std::fs::remove_dir_all(draft.directory);
    for candidate in draft.extension_candidates {
        if candidate.candidate_id != request.candidate_id {
            let _ =
                super::extension_activation::discard(&candidate.base_id, &candidate.candidate_id);
        }
    }
    super::clear_orchestrator_pet_cache();
    let snapshot = app
        .state::<crate::preferences::PreferencesStore>()
        .snapshot();
    let _ = app.emit("pet-studio-preferences", &snapshot);
    let reload_warning =
        crate::street_windows::emit_to_streets(app.clone(), "user-packs-changed", ())
            .err()
            .map(|_| {
                "The extension is active, but the village could not reload it automatically."
                    .to_string()
            });
    Ok(ActivateExtensionResult { id, reload_warning })
}

#[tauri::command]
pub fn discard_pet_extension_candidate(
    request: DiscardExtensionCandidateRequest,
    state: tauri::State<'_, PetStudioState>,
) -> Result<(), String> {
    let mut drafts = state.0.lock().unwrap_or_else(|error| error.into_inner());
    super::extension_activation::discard(&request.base_id, &request.candidate_id)?;
    for draft in drafts.values_mut() {
        draft
            .extension_candidates
            .retain(|candidate| candidate.candidate_id != request.candidate_id);
    }
    Ok(())
}
