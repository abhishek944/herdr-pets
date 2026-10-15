use super::state::OrchestratorState;
use super::status_types::OrchestratorPetState;
use crate::preferences::PreferencesStore;
use std::sync::Mutex;
use tauri::Manager;
use tauri::{AppHandle, Emitter};

static MAYOR_PUBLICATION: Mutex<()> = Mutex::new(());

pub fn publish(state: &OrchestratorState, app: &AppHandle) {
    let _ = app.emit("orchestrator-status-refresh", ());
    let pet = state.pet_state(app);
    let _ = crate::street_windows::emit_to_streets(app.clone(), "orchestrator-pet-state", pet);
    publish_mayor(state, app);
}

pub fn publish_mayor(state: &OrchestratorState, app: &AppHandle) {
    let _publication = MAYOR_PUBLICATION
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let pet = state.pet_state(app);
    let runtime = state.0.lock().unwrap_or_else(|error| error.into_inner());
    let serial = runtime.mayor_focus_serial;
    let firstmate_mode = app
        .state::<PreferencesStore>()
        .snapshot()
        .preferences
        .app
        .orchestrator
        .mode
        == crate::preferences_model::MayorMode::Firstmate;
    let conversation_active = if firstmate_mode {
        super::firstmate::phase(app) != super::firstmate::READY
    } else {
        runtime.listening
            || runtime.mayor_speaking
            || runtime.live_connected
            || runtime.connecting
            || runtime.wake_activated
    };
    let speech = runtime.mayor_speech.clone();
    let voice_status = runtime.mayor_voice_status.clone();
    drop(runtime);
    if let Ok(path) = crate::preferences_io::preferences_path() {
        let path = path.with_file_name("mayor-state.json");
        let temporary = path.with_file_name(format!("mayor-state.{}.tmp", std::process::id()));
        let payload = serde_json::json!({
            "ownerPid": std::process::id(),
            "active": pet.active,
            "name": pet.display_name,
            "listening": pet.listening,
            "working": pet.working,
            "focusSerial": serial,
            "conversationActive": conversation_active,
            "speech": speech,
            "voiceStatus": voice_status,
            "firstmateMode": firstmate_mode,
            "firstmateOwned": super::firstmate::firstmate_voice_ready(app.clone()),
            "speaking": pet.speaking,
        });
        if let Ok(bytes) = serde_json::to_vec(&payload) {
            if std::fs::write(&temporary, bytes).is_ok() {
                let _ = std::fs::rename(&temporary, &path);
            }
        }
    }
}

pub fn pet_state(state: &OrchestratorState, app: &AppHandle) -> OrchestratorPetState {
    let preferences = app.state::<PreferencesStore>().snapshot().preferences;
    let configured = &preferences.app.orchestrator;
    let firstmate = configured.mode == crate::preferences_model::MayorMode::Firstmate;
    let phase = if firstmate {
        super::firstmate::phase(app)
    } else {
        super::firstmate::READY
    };
    let pet_ready = crate::pet_studio::orchestrator_pet_ready(configured.pet_id.as_deref());
    let runtime = state.0.lock().unwrap_or_else(|error| error.into_inner());
    OrchestratorPetState {
        active: configured.enabled && pet_ready,
        citizen_id: Some("pet-town-assistant".into()),
        pet_id: configured.pet_id.clone(),
        display_name: configured.display_name.clone(),
        listening: if firstmate {
            phase == super::firstmate::LISTENING
        } else {
            runtime.listening
        },
        working: if firstmate {
            phase == super::firstmate::WORKING
        } else {
            runtime.active_task_id.is_some()
        },
        speaking: if firstmate {
            phase == super::firstmate::SPEAKING
        } else {
            runtime.mayor_speaking
        },
    }
}

impl OrchestratorState {
    pub fn set_mayor_speech(&self, text: String, speaking: bool, app: &AppHandle) {
        let mut runtime = self.0.lock().unwrap_or_else(|error| error.into_inner());
        runtime.mayor_speech = text.chars().take(2400).collect();
        let changed = runtime.mayor_speaking != speaking;
        runtime.mayor_speaking = speaking;
        drop(runtime);
        if changed {
            let _ = crate::street_windows::emit_to_streets(
                app.clone(),
                "orchestrator-pet-state",
                self.pet_state(app),
            );
        }
        publish_mayor(self, app);
    }

    pub fn pet_state(&self, app: &AppHandle) -> OrchestratorPetState {
        pet_state(self, app)
    }

    pub fn emit(&self, app: &AppHandle) {
        publish(self, app);
    }

    pub fn focus_mayor(&self, app: &AppHandle) {
        let mut runtime = self.0.lock().unwrap_or_else(|error| error.into_inner());
        runtime.mayor_focus_serial = runtime.mayor_focus_serial.wrapping_add(1);
        drop(runtime);
        self.emit(app);
    }
}
