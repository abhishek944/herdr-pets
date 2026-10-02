#[derive(Clone, Debug)]
pub(crate) enum FocusRoute {
    Herdr {
        pane_id: String,
        socket: Option<String>,
        machine: Option<String>,
        agent_session_id: String,
    },
    Application {
        bundle_id: String,
    },
    Codex {
        thread_id: Option<String>,
        fallback_bundle_id: Option<String>,
    },
}

impl From<pet_town_agent_broker::FocusRoute> for FocusRoute {
    fn from(route: pet_town_agent_broker::FocusRoute) -> Self {
        match route {
            pet_town_agent_broker::FocusRoute::Herdr {
                pane_id,
                socket,
                machine,
                agent_session_id,
            } => Self::Herdr {
                pane_id,
                socket,
                machine,
                agent_session_id,
            },
            pet_town_agent_broker::FocusRoute::Application { bundle_id } => {
                Self::Application { bundle_id }
            }
            pet_town_agent_broker::FocusRoute::Codex {
                thread_id,
                fallback_bundle_id,
            } => Self::Codex {
                thread_id,
                fallback_bundle_id,
            },
        }
    }
}
