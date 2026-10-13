export const ASSISTANT_PET_ID = "knight";

export type SettingsAppearance = "system" | "light" | "dark";
export type StripTheme =
  | "standard"
  | "ocean"
  | "rainforest"
  | "snowy"
  | "desert"
  | "rollercoaster"
  | "goldmine"
  | "railway";
export type CoasterMode = "low-rolling" | "twilight-carnival" | "firefly-gardens" | "cloudline";
export function isActivityTheme(theme: string | undefined): boolean {
  return theme === "rollercoaster" || theme === "goldmine" || theme === "railway";
}
export type PetTheme = "standard" | "ocean";
export type RainforestMode = "after-rain" | "firefly";
export type SnowMode = "fresh-snow" | "aurora-night";
export type DesertScene = "palm-spring" | "adobe-outpost" | "lantern-caravan";
export type DesertMode = "golden-dunes" | "moonlit-oasis";
export type LabelVisibility = "always" | "hover" | "hidden";
export type MotionLevel = "gentle" | "standard" | "playful";

export interface AppPreferences {
  openWithHerdr: boolean;
  settingsAppearance: SettingsAppearance;
  stripTheme: StripTheme;
  coasterMode: CoasterMode;
  coasterHeightPx: number;
  goldmineOpacityPercent: number;
  railwayOpacityPercent: number;
  rainforestMode: RainforestMode;
  rainforestOpacityPercent: number;
  snowMode: SnowMode;
  snowOpacityPercent: number;
  desertScene: DesertScene;
  desertMode: DesertMode;
  desertOpacityPercent: number;
  oceanOpacityPercent: number;
  oceanWaterlineHeightPx: number;
  lastSelectedPetId: string;
  hideCompletedPets: boolean;
  completedHideDelayMinutes: number;
  orchestrator: OrchestratorPreferences;
}

export interface OrchestratorPreferences {
  mode: "firstmate" | "live";
  firstmatePath: string | null;
  trustedFirstmatePath: string | null;
  enabled: boolean;
  displayName: string;
  model: "gpt-5.6-luna" | "gpt-5.6-sol" | "gpt-5.6-terra";
  thinking: "low" | "medium" | "high" | "xhigh";
  wakeEnabled: boolean;
  petId: string | null;
  workspaceId: string | null;
  systemPrompt: string;
}

export interface PetPreferences {
  customName: string;
  theme: PetTheme;
  includedInRandomCast: boolean;
  appearance: { scalePercent: number; opacityPercent: number };
  labels: { visibility: LabelVisibility; textScalePercent: number };
  motion: { level: MotionLevel; pauseOnHover: boolean };
}

export interface PreferencesFile {
  schemaVersion: number;
  app: AppPreferences;
  pets: Record<string, PetPreferences>;
}

export interface PreferencesSnapshot {
  revision: number;
  preferences: PreferencesFile;
  defaults: PreferencesFile;
  petIds: string[];
  warning: string | null;
  readOnly: boolean;
}

export interface SettingsContext {
  /** Null means a general Settings entry, rather than an explicit pet. */
  selectedPetId: string | null;
  /** "app" opens Settings on the App tab with the Preview switch. Null keeps the current tab. */
  initialTab: string | null;
}

export function clonePreferences(value: PreferencesFile): PreferencesFile {
  return structuredClone(value);
}

export function preferencesEqual(left: PreferencesFile, right: PreferencesFile): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function friendlyPetName(id: string): string {
  return id
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function motionFactor(level: MotionLevel): number {
  if (level === "gentle") return 0.7;
  if (level === "playful") return 1.25;
  return 1;
}
