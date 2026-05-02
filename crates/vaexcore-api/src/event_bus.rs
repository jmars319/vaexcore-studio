use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tokio::sync::broadcast;
use vaexcore_core::StudioEvent;

const RECENT_EVENT_LIMIT: usize = 100;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<StudioEvent>,
    recent: Arc<Mutex<VecDeque<StudioEvent>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            sender,
            recent: Arc::new(Mutex::new(VecDeque::with_capacity(RECENT_EVENT_LIMIT))),
        }
    }

    pub fn emit(&self, event: StudioEvent) {
        {
            let mut recent = self.recent.lock().expect("event bus mutex poisoned");
            if recent.len() == RECENT_EVENT_LIMIT {
                recent.pop_front();
            }
            recent.push_back(event.clone());
        }

        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StudioEvent> {
        self.sender.subscribe()
    }

    pub fn recent(&self) -> Vec<StudioEvent> {
        self.recent
            .lock()
            .expect("event bus mutex poisoned")
            .iter()
            .cloned()
            .collect()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
