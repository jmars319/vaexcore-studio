use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use vaexcore_core::{new_id, now_utc, ConnectedClient};

const RECENT_CLIENT_LIMIT: usize = 50;

#[derive(Clone, Debug)]
pub struct ClientSeen {
    pub client_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Default)]
pub struct ClientRegistry {
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, seen: ClientSeen) {
        let now = now_utc();
        let key = client_key(&seen);
        let mut clients = self.clients.lock().expect("client registry mutex poisoned");

        match clients.get_mut(&key) {
            Some(client) => {
                client.name = seen.name;
                client.kind = seen.kind;
                client.user_agent = seen.user_agent;
                client.last_request_id = seen.request_id;
                client.last_path = seen.path;
                client.request_count += 1;
                client.last_seen_at = now;
            }
            None => {
                clients.insert(
                    key,
                    ConnectedClient {
                        id: seen.client_id.unwrap_or_else(|| new_id("client")),
                        name: seen.name,
                        kind: seen.kind,
                        user_agent: seen.user_agent,
                        last_request_id: seen.request_id,
                        last_path: seen.path,
                        request_count: 1,
                        connected_at: now,
                        last_seen_at: now,
                    },
                );
            }
        }

        if clients.len() > RECENT_CLIENT_LIMIT {
            let mut ordered = clients
                .iter()
                .map(|(key, client)| (key.clone(), client.last_seen_at))
                .collect::<Vec<_>>();
            ordered.sort_by_key(|(_, last_seen_at)| *last_seen_at);
            for (key, _) in ordered
                .into_iter()
                .take(clients.len() - RECENT_CLIENT_LIMIT)
            {
                clients.remove(&key);
            }
        }
    }

    pub fn recent(&self) -> Vec<ConnectedClient> {
        let mut clients = self
            .clients
            .lock()
            .expect("client registry mutex poisoned")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        clients.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
        clients
    }
}

fn client_key(seen: &ClientSeen) -> String {
    seen.client_id.clone().unwrap_or_else(|| {
        format!(
            "{}:{}:{}",
            seen.kind,
            seen.name,
            seen.user_agent.clone().unwrap_or_default()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_registry_updates_existing_client() {
        let registry = ClientRegistry::new();
        registry.register(ClientSeen {
            client_id: Some("client-a".to_string()),
            name: "Client A".to_string(),
            kind: "http".to_string(),
            user_agent: Some("test-agent".to_string()),
            request_id: Some("req_1".to_string()),
            path: Some("/status".to_string()),
        });
        registry.register(ClientSeen {
            client_id: Some("client-a".to_string()),
            name: "Client A".to_string(),
            kind: "http".to_string(),
            user_agent: Some("test-agent".to_string()),
            request_id: Some("req_2".to_string()),
            path: Some("/profiles".to_string()),
        });

        let clients = registry.recent();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].request_count, 2);
        assert_eq!(clients[0].last_request_id.as_deref(), Some("req_2"));
    }
}
