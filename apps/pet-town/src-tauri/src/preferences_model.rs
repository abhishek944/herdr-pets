mod presentation;
mod scenery;
pub use scenery::*;

pub use presentation::{
    AppearancePreferences, CoasterMode, DesertMode, DesertScene, LabelPreferences, LabelVisibility,
    MotionLevel, MotionPreferences, PetPreferences, PetTheme, RainforestMode, SettingsAppearance,
    SnowMode, StripTheme,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 3;
pub const ASSISTANT_PET_ID: &str = "knight";
pub const COMPLETED_HIDE_DELAYS_MINUTES: [u16; 5] = [1, 5, 15, 30, 60];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferencesFile {
    pub schema_version: u32,
    pub app: AppPreferences,
    pub pets: BTreeMap<String, PetPreferences>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppPreferences {
    pub open_with_herdr: bool,
    pub settings_appearance: SettingsAppearance,
    #[serde(default)]
    pub strip_theme: StripTheme,
    #[serde(default)]
    pub coaster_mode: CoasterMode,
    #[serde(default = "default_coaster_height_px")]
    pub coaster_height_px: u16,
    #[serde(default = "default_activity_opacity_percent")]
    pub goldmine_opacity_percent: u8,
    #[serde(default = "default_activity_opacity_percent")]
    pub railway_opacity_percent: u8,
    #[serde(default)]
    pub rainforest_mode: RainforestMode,
    #[serde(default = "default_rainforest_opacity_percent")]
    pub rainforest_opacity_percent: u8,
    #[serde(default)]
    pub snow_mode: SnowMode,
    #[serde(default = "default_snow_opacity_percent")]
    pub snow_opacity_percent: u8,
    #[serde(default)]
    pub desert_scene: DesertScene,
    #[serde(default)]
    pub desert_mode: DesertMode,
    #[serde(default = "default_desert_opacity_percent")]
    pub desert_opacity_percent: u8,
    #[serde(default = "default_ocean_opacity_percent")]
    pub ocean_opacity_percent: u8,
    #[serde(default = "default_ocean_waterline_height_px")]
    pub ocean_waterline_height_px: u16,
    pub last_selected_pet_id: String,
    pub hide_completed_pets: bool,
    pub completed_hide_delay_minutes: u16,
    pub orchestrator: OrchestratorPreferences,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestratorPreferences {
    #[serde(default)]
    pub mode: MayorMode,
    #[serde(default)]
    pub firstmate_path: Option<String>,
    #[serde(default)]
    pub trusted_firstmate_path: Option<String>,
    pub enabled: bool,
    pub display_name: String,
    pub model: String,
    pub thinking: String,
    pub wake_enabled: bool,
    pub pet_id: Option<String>,
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub system_prompt: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MayorMode {
    #[default]
    Firstmate,
    Live,
}

pub fn bundled_pet_ids() -> Vec<String> {
    env!("PET_TOWN_PET_IDS")
        .split(',')
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

impl PreferencesFile {
    pub fn defaults(ids: &[String]) -> Self {
        let first = ids
            .iter()
            .find(|id| id.as_str() == ASSISTANT_PET_ID)
            .or_else(|| ids.first())
            .cloned()
            .unwrap_or_else(|| ASSISTANT_PET_ID.to_string());
        Self {
            schema_version: SCHEMA_VERSION,
            app: AppPreferences {
                open_with_herdr: true,
                settings_appearance: SettingsAppearance::System,
                strip_theme: StripTheme::Standard,
                coaster_mode: CoasterMode::LowRolling,
                coaster_height_px: default_coaster_height_px(),
                goldmine_opacity_percent: default_activity_opacity_percent(),
                railway_opacity_percent: default_activity_opacity_percent(),
                rainforest_mode: RainforestMode::AfterRain,
                rainforest_opacity_percent: default_rainforest_opacity_percent(),
                snow_mode: SnowMode::FreshSnow,
                snow_opacity_percent: default_snow_opacity_percent(),
                desert_scene: DesertScene::PalmSpring,
                desert_mode: DesertMode::GoldenDunes,
                desert_opacity_percent: default_desert_opacity_percent(),
                ocean_opacity_percent: default_ocean_opacity_percent(),
                ocean_waterline_height_px: default_ocean_waterline_height_px(),
                last_selected_pet_id: first,
                hide_completed_pets: false,
                completed_hide_delay_minutes: 5,
                orchestrator: OrchestratorPreferences {
                    mode: MayorMode::Firstmate,
                    firstmate_path: None,
                    trusted_firstmate_path: None,
                    enabled: false,
                    display_name: "Mayor".to_string(),
                    model: "gpt-5.6-luna".to_string(),
                    thinking: "medium".to_string(),
                    wake_enabled: true,
                    pet_id: Some(ASSISTANT_PET_ID.to_string()),
                    workspace_id: None,
                    system_prompt: String::new(),
                },
            },
            pets: ids
                .iter()
                .map(|id| (id.clone(), PetPreferences::default()))
                .collect(),
        }
    }

    pub fn normalize_assistant(&mut self) {
        self.app.orchestrator.wake_enabled = true;
        self.app.orchestrator.pet_id = Some(ASSISTANT_PET_ID.to_string());
    }
}

#[cfg(test)]
mod mayor_name_tests {
    use super::PreferencesFile;

    #[test]
    fn pet_names_survive_save_and_older_preferences_still_load() {
        let ids = vec!["knight".to_string()];
        let mut preferences = PreferencesFile::defaults(&ids);
        preferences.pets.get_mut("knight").unwrap().custom_name = "Pip".into();
        let value = serde_json::to_value(&preferences).unwrap();
        let restored: PreferencesFile = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(restored.pets["knight"].custom_name, "Pip");
        assert!(restored.validate(&ids).is_ok());

        let mut older = value;
        older["pets"]["knight"]
            .as_object_mut()
            .unwrap()
            .remove("customName");
        let restored: PreferencesFile = serde_json::from_value(older).unwrap();
        assert!(restored.pets["knight"].custom_name.is_empty());
        assert!(restored.validate(&ids).is_ok());
    }
}
