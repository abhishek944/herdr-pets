use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetPreferences {
    #[serde(default)]
    pub custom_name: String,
    #[serde(default)]
    pub theme: PetTheme,
    pub included_in_random_cast: bool,
    pub appearance: AppearancePreferences,
    pub labels: LabelPreferences,
    pub motion: MotionPreferences,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppearancePreferences {
    pub scale_percent: u16,
    pub opacity_percent: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LabelPreferences {
    pub visibility: LabelVisibility,
    pub text_scale_percent: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionPreferences {
    pub level: MotionLevel,
    // Legacy per-pet "reduce movement" toggle, removed from Settings and never
    // written to new files. Kept readable so older preference files still load.
    #[serde(default, skip_serializing)]
    pub reduced: bool,
    pub pause_on_hover: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsAppearance {
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StripTheme {
    #[default]
    Standard,
    Ocean,
    Rainforest,
    Snowy,
    Desert,
    Rollercoaster,
    Goldmine,
    Railway,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoasterMode {
    #[default]
    LowRolling,
    TwilightCarnival,
    FireflyGardens,
    Cloudline,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PetTheme {
    #[default]
    Standard,
    Ocean,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RainforestMode {
    #[default]
    AfterRain,
    Firefly,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnowMode {
    #[default]
    FreshSnow,
    AuroraNight,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesertScene {
    #[default]
    PalmSpring,
    AdobeOutpost,
    LanternCaravan,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesertMode {
    #[default]
    GoldenDunes,
    MoonlitOasis,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelVisibility {
    Always,
    Hover,
    Hidden,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MotionLevel {
    Gentle,
    Standard,
    Playful,
}
