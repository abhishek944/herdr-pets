use crate::preferences_model::{
    bundled_pet_ids, PetTheme, PreferencesFile, ASSISTANT_PET_ID, COMPLETED_HIDE_DELAYS_MINUTES,
    SCHEMA_VERSION,
};

impl PreferencesFile {
    pub fn validate(&self, ids: &[String]) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("unsupported preference schema version".to_string());
        }
        if !self.pets.contains_key(&self.app.last_selected_pet_id) {
            return Err("lastSelectedPetId is not an installed pet".to_string());
        }
        if !(50..=146).contains(&self.app.coaster_height_px) {
            return Err("coasterHeightPx must be 50–146".to_string());
        }
        if self.app.goldmine_opacity_percent > 100 {
            return Err("goldmineOpacityPercent must be 0–100".to_string());
        }
        if self.app.railway_opacity_percent > 100 {
            return Err("railwayOpacityPercent must be 0–100".to_string());
        }
        if self.app.rainforest_opacity_percent > 100 {
            return Err("rainforestOpacityPercent must be 0–100".to_string());
        }
        if self.app.snow_opacity_percent > 100 {
            return Err("snowOpacityPercent must be 0–100".to_string());
        }
        if self.app.desert_opacity_percent > 100 {
            return Err("desertOpacityPercent must be 0–100".to_string());
        }
        if !(16..=36).contains(&self.app.ocean_opacity_percent) {
            return Err("oceanOpacityPercent must be 16–36".to_string());
        }
        if !(50..=146).contains(&self.app.ocean_waterline_height_px) {
            return Err("oceanWaterlineHeightPx must be 50–146".to_string());
        }
        if !COMPLETED_HIDE_DELAYS_MINUTES.contains(&self.app.completed_hide_delay_minutes) {
            return Err("completedHideDelayMinutes is not a supported delay".to_string());
        }
        let orchestrator = &self.app.orchestrator;
        let name = orchestrator.display_name.trim();
        if name.is_empty() || name.chars().count() > 32 || name.chars().any(char::is_control) {
            return Err("orchestrator.displayName must contain 1–32 safe characters".to_string());
        }
        if !["gpt-5.6-luna", "gpt-5.6-sol", "gpt-5.6-terra"].contains(&orchestrator.model.as_str())
            || !["low", "medium", "high", "xhigh"].contains(&orchestrator.thinking.as_str())
        {
            return Err("orchestrator model or thinking level is unsupported".to_string());
        }
        if !orchestrator.wake_enabled {
            return Err("orchestrator.wakeEnabled must remain on".to_string());
        }
        if let Some(path) = orchestrator.firstmate_path.as_deref() {
            if path.is_empty()
                || path.len() > 4096
                || !std::path::Path::new(path).is_absolute()
                || path.chars().any(char::is_control)
            {
                return Err(
                    "orchestrator.firstmatePath must be an absolute safe folder path".to_string(),
                );
            }
        }
        if orchestrator.trusted_firstmate_path.is_some()
            && orchestrator.trusted_firstmate_path != orchestrator.firstmate_path
        {
            return Err("Firstmate trust must match the selected folder".into());
        }
        if let Some(id) = orchestrator.workspace_id.as_deref() {
            if id.is_empty() || id.len() > 1024 {
                return Err("orchestrator.workspaceId must be 1–1024 characters".to_string());
            }
        }
        if orchestrator.system_prompt.chars().count() > 4_000
            || orchestrator
                .system_prompt
                .chars()
                .any(|character| character.is_control() && !"\n\r\t".contains(character))
        {
            return Err(
                "orchestrator.systemPrompt must be at most 4000 safe characters".to_string(),
            );
        }
        if orchestrator.pet_id.as_deref() != Some(ASSISTANT_PET_ID)
            || !ids.iter().any(|id| id == ASSISTANT_PET_ID)
        {
            return Err("orchestrator.petId must use the bundled Knight".to_string());
        }
        let actual: Vec<&str> = self.pets.keys().map(String::as_str).collect();
        let expected: Vec<&str> = ids.iter().map(String::as_str).collect();
        if actual != expected {
            return Err("pets must contain every installed pet ID exactly once".to_string());
        }
        if !self.pets.values().any(|pet| pet.included_in_random_cast) {
            return Err("at least one pet must be included in the random cast".to_string());
        }
        let ocean_pets = bundled_pet_ids();
        for (id, pet) in &self.pets {
            if pet.theme == PetTheme::Ocean && !ocean_pets.contains(id) {
                return Err(format!("{id}.theme has no Ocean animations"));
            }
            let custom_name = &pet.custom_name;
            if custom_name.chars().count() > 32 || custom_name.chars().any(char::is_control) {
                return Err(format!(
                    "{id}.customName must contain at most 32 safe characters"
                ));
            }
            if !(75..=175).contains(&pet.appearance.scale_percent) {
                return Err(format!("{id}.appearance.scalePercent must be 75–175"));
            }
            if !(30..=100).contains(&pet.appearance.opacity_percent) {
                return Err(format!("{id}.appearance.opacityPercent must be 30–100"));
            }
            if !(75..=160).contains(&pet.labels.text_scale_percent) {
                return Err(format!("{id}.labels.textScalePercent must be 75–160"));
            }
        }
        Ok(())
    }
}
