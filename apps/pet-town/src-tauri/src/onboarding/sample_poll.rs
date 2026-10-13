use super::{model::Sample, Onboarding};
use pet_town_agent_broker::FocusRoute;

pub(super) fn poll(state: &Onboarding) {
    let progress = state.store.get();
    let mut owner = if progress.own_baseline.is_some() {
        None
    } else {
        progress.sample
    };
    if let Some(saved) = owner.as_mut() {
        if saved.launch_attempted {
            if super::sample_identity::verify(saved).is_err() {
                return;
            }
            if let Ok(value) =
                super::process::owned(saved, &["agent", "get", &super::sample::name(saved)], 5)
            {
                let agent = &value["result"]["agent"];
                if agent["pane_id"].as_str() == saved.pane.as_deref()
                    && agent["workspace_id"].as_str() == saved.workspace.as_deref()
                {
                    let session = agent["agent_session"]["value"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            format!("ephemeral:{}", saved.pane.as_deref().unwrap_or(""))
                        });
                    if saved
                        .session
                        .as_ref()
                        .is_none_or(|old| old == &session || old.starts_with("ephemeral:"))
                        && saved.session.as_ref() != Some(&session)
                    {
                        saved.session = Some(session);
                        let updated = saved.clone();
                        if state.store.update(|p| p.sample = Some(updated)).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    }
    if owner.is_none() && progress.own_baseline.is_none() {
        return;
    }
    let broker = crate::sessions::collect_visible();
    let candidate = broker.snapshot.agents.iter().find(|agent| {
        if let Some(owner) = &owner {
            matches!(broker.focus_routes.get(&agent.id), Some(FocusRoute::Herdr { pane_id, agent_session_id, socket, machine: None, .. })
                if socket == &owner.socket && Some(pane_id) == owner.pane.as_ref() && owner.session.as_ref().is_some_and(|session| session == agent_session_id))
        } else {
            agent.source == "herdr" && progress.own_baseline.as_ref().is_some_and(|ids| !ids.contains(&agent.id))
        }
    });
    let Some(agent) = candidate else {
        return;
    };
    let mut runtime = state.runtime();
    let visible = runtime.rendered_ids.contains(&agent.id)
        && runtime
            .rendered_at
            .is_some_and(|at| at.elapsed().as_secs() < 3);
    let submitted = owner.as_ref().is_none_or(|owner| owner.prompt_attempted);
    let arrived = visible && submitted && matches!(agent.status.as_str(), "working" | "done");
    if arrived
        && owner.as_ref().is_some_and(|owner| !owner.arrival_seen)
        && state
            .store
            .update(|p| {
                if let Some(owner) = &mut p.sample {
                    owner.arrival_seen = true;
                }
            })
            .is_err()
    {
        return;
    }
    let phase = if agent.status == "blocked" {
        "blocked"
    } else if arrived
        || owner.as_ref().is_some_and(|owner| owner.arrival_seen)
        || (owner.is_none()
            && runtime.sample.phase == "arrived"
            && runtime.sample.agent_id.as_deref() == Some(&agent.id))
    {
        "arrived"
    } else {
        "waiting"
    };
    runtime.sample = Sample {
        phase: phase.into(),
        status: Some(agent.status.clone()),
        agent_id: Some(agent.id.clone()),
        message: if agent.status == "blocked" {
            Some(if owner.is_some() { "Your coding tool needs sign-in, trust or approval. Finish that step in Herdr, then retry if the hello has not been sent." } else { "Your coding tool needs input. Finish sign-in, trust or approval in Herdr. Pet Town will keep watching." }.into())
        } else {
            None
        },
        owned: owner.is_some(),
    };
}
