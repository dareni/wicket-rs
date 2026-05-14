use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex;

use crate::components::WebPage;
use dashmap::DashMap;
use rand::random;

pub mod page_factory;

const FIVE_MIN_SECS: u16 = 300;

/// Application sessions container.
pub struct SessionRegistry {
    // Epoch seconds.
    app_start: u64,
    // Key: SessionId u32
    sessions: DashMap<u32, Arc<Mutex<SessionData>>>,
}

pub struct SessionData {
    last_touched: u16,
    // Key: PageId (u16) -> Value: History of that page
    pages: HashMap<u16, Vec<Box<dyn WebPage>>>,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self {
            app_start: Self::calc_app_start(),
            sessions: DashMap::new(),
        }
    }
}

impl SessionRegistry {
    pub fn calc_app_start() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }

    fn get_current_5min_tick(&self) -> u16 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        ((now - self.app_start) / u64::from(FIVE_MIN_SECS)) as u16
    }

    pub fn new_session(&self) -> u32 {
        let last_touched = self.get_current_5min_tick();
        loop {
            let session_id: u32 = random();
            let new_data = SessionData {
                last_touched,
                pages: HashMap::new(),
            };

            if self
                .sessions
                .insert(session_id, Arc::from(Mutex::new(new_data)))
                .is_none()
            {
                return session_id;
            }
        }
    }

    pub fn get_session_handle(&self, session_id: u32) -> Option<Arc<Mutex<SessionData>>> {
        let handle = {
            let dash_shard_handle = self.sessions.get(&session_id)?;
            let session_mut = dash_shard_handle.value();
            Arc::clone(session_mut)
            // drop the dash_shard_handle asap.
        };
        Some(handle)
    }

    pub async fn with<F, Fut, R>(&self, session_id: u32, f: F) -> Option<R>
    where
        F: FnOnce(Arc<Mutex<SessionData>>) -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let session_handle = self.sessions.get(&session_id)?.clone();
        let local_session_handle = session_handle.clone();
        let future = Some(f(session_handle).await);
        let mut session = local_session_handle.lock().await;
        session.last_touched = self.get_current_5min_tick();
        future
    }
}

impl SessionData {
    /// Session refresh time in sec/300 relative to the server start.
    pub fn get_last_touched(&self) -> u16 {
        self.last_touched
    }

    pub fn get_page(&self, page_instance: u16, page_version: u16) -> Option<&dyn WebPage> {
        self.pages
            .get(&page_instance)
            .and_then(|vec| vec.get(page_version as usize))
            .map(|boxed| &**boxed)
    }
}
