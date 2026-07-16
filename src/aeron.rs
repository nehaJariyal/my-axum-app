//! Publishes signup events to Aeron via the aeron-viewer HTTP bridge.

use reqwest::Client;
use serde::Serialize;

use crate::models::user::User;

#[derive(Clone)]
pub struct AeronPublisher {
    client: Client,
    url: String,
    
}

#[derive(Serialize)]
struct SignupEvent<'a> {
    event: &'static str,
    user: &'a User,
}

impl AeronPublisher {
    pub fn new(publish_url: String) -> Self {
        Self {
            client: Client::new(),
            url: publish_url,
        }
    }

    pub async fn publish_signup(&self, user: &User) {
        let payload = match serde_json::to_string(&SignupEvent {
            event: "user.signup",
            user,
        }) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("[aeron] failed to serialize signup event: {err}");
                return;
            }
        };

        match self
            .client
            .post(&self.url)
            .header("Content-Type", "text/plain")
            .body(payload)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("[aeron] published signup for user_id={}", user.id);
            }
            Ok(response) => {
                eprintln!(
                    "[aeron] publish failed for user_id={} status={}",
                    user.id,
                    response.status()
                );
            }
            Err(err) => eprintln!("[aeron] publish request failed for user_id={}: {err}", user.id),
        }
    }
}
