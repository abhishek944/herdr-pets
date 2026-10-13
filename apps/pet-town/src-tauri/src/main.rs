// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    #[cfg(unix)]
    if arguments.iter().any(|argument| argument == "--open-town") {
        if arguments.len() != 2 {
            eprintln!("usage: pet-town --open-town");
            std::process::exit(2);
        }
        if let Err(message) = pet_town_lib::open_town_from_cli() {
            eprintln!("{message}");
            std::process::exit(2);
        }
        return;
    }
    if arguments
        .iter()
        .any(|argument| argument == "--onboarding-herdr")
    {
        if let Err(message) = pet_town_lib::open_onboarding_herdr_from_cli() {
            eprintln!("{message}");
            std::process::exit(2);
        }
        println!("ok");
        return;
    }
    if arguments
        .iter()
        .any(|argument| argument == "--usage-snapshot")
    {
        println!("{}", pet_town_lib::codex_usage_snapshot_from_cli());
        return;
    }
    if arguments.iter().any(|argument| argument == "--snapshot") {
        println!("{}", pet_town_lib::snapshot_json());
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--focus-native-town")
    {
        let result = arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| "--focus-native-town requires the town PID and desktop PID".to_string())
            .and_then(|(pid, parent)| {
                let pid = pid
                    .parse::<u32>()
                    .map_err(|_| "Invalid town PID".to_string())?;
                let parent = parent
                    .parse::<u32>()
                    .map_err(|_| "Invalid desktop PID".to_string())?;
                pet_town_lib::focus_native_town_from_cli(pid, parent)
            });
        if let Err(message) = result {
            eprintln!("{message}");
            std::process::exit(2);
        }
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--focus-agent")
    {
        let result = arguments
            .get(index + 1)
            .ok_or_else(|| "--focus-agent requires an agent ID".to_string())
            .and_then(|id| pet_town_lib::focus_agent_from_cli(id));
        if let Err(message) = result {
            eprintln!("{message}");
            std::process::exit(2);
        }
        return;
    }
    if arguments
        .iter()
        .any(|argument| argument == "--startup-enabled")
    {
        std::process::exit(if pet_town_lib::startup_enabled_from_disk() {
            0
        } else {
            1
        });
    }
    if arguments
        .iter()
        .any(|argument| argument == "--adapter-statuses")
    {
        println!(
            "{}",
            pet_town_lib::setups_json().unwrap_or_else(|_| "[]".to_string())
        );
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--configure-adapter")
    {
        let result = arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| {
                "--configure-adapter requires an adapter ID and enable or disable".to_string()
            })
            .and_then(|(id, action)| match action.as_str() {
                "enable" => pet_town_lib::configure_adapter(id, true),
                "disable" => pet_town_lib::configure_adapter(id, false),
                _ => Err("adapter action must be enable or disable".to_string()),
            });
        if let Err(message) = result {
            eprintln!("{message}");
            std::process::exit(2);
        }
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--verify-existing-application")
    {
        let result = arguments
            .get(index + 1)
            .ok_or_else(|| "--verify-existing-application requires a bundle ID".to_string())
            .and_then(|bundle| pet_town_lib::verify_existing_application(bundle.clone()));
        if let Err(message) = result {
            eprintln!("{message}");
            std::process::exit(2);
        }
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--verify-existing-herdr-agent")
    {
        let result = arguments
            .get(index + 1)
            .ok_or_else(|| "--verify-existing-herdr-agent requires a pane ID".to_string())
            .and_then(|pane| pet_town_lib::verify_existing_herdr_agent(pane.clone()));
        if let Err(message) = result {
            eprintln!("{message}");
            std::process::exit(2);
        }
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--adapter-relay")
    {
        // The relay strips sensitive fields before handing off to the detached
        // record writer. It always fails open for the calling harness.
        let _ = arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| "--adapter-relay requires a source and event name".to_string())
            .and_then(|(source, event)| {
                pet_town_lib::relay_from_stdin(
                    source,
                    event,
                    arguments.get(index + 4).map(String::as_str),
                )
            });
        println!("{{}}");
        return;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--adapter-event")
    {
        // Adapter hooks are observation-only and must always fail open. A malformed
        // or unavailable event is ignored so it can never interrupt the harness.
        let _ = arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| "--adapter-event requires a source and event name".to_string())
            .and_then(|(source, event)| {
                pet_town_lib::record_from_stdin(
                    source,
                    event,
                    arguments.get(index + 4).map(String::as_str),
                )
            });
        println!("{{}}");
        return;
    }
    pet_town_lib::run();
}
