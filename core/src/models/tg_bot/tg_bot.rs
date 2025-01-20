use std::sync::Arc;
use tokio::sync::{watch, Mutex};

pub struct PodcastManager {
    pub state: Arc<PodcastTaskState>,
    pub stop_rx: watch::Receiver<bool>,
}

impl PodcastManager {
    pub fn new() -> Self {
        let (state, stop_rx) = PodcastTaskState::new();
        Self {
            state: Arc::new(state),
            stop_rx,
        }
    }
}

#[derive(Clone)]
pub struct PodcastTaskState {
    pub is_running: Arc<Mutex<bool>>,
    pub stop_sender: watch::Sender<bool>,
}

impl PodcastTaskState {
    pub fn new() -> (Self, watch::Receiver<bool>) {
        let (stop_sender, stop_receiver) = watch::channel(false);
        (
            Self {
                is_running: Arc::new(Mutex::new(false)),
                stop_sender,
            },
            stop_receiver,
        )
    }
}
