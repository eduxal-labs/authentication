use crate::types::Error;
use reqwest::Client;
use serde_json::json;

const URL: &str = "https://graph.facebook.com/v21.0/960426547146856/messages";

pub struct Messenger {
    token: &'static str,
    client: Client,
}

impl Default for Messenger {
    fn default() -> Self {
        let token = env!("WHATSAPP_TOKEN");
        let client = Client::new();
        Self { token, client }
    }
}

impl Messenger {
    fn body(receipient: &str, code: &str) -> serde_json::Value {
        json!({
            "messaging_product": "whatsapp",
            "to": receipient,
            "type": "template",
            "template": {
                "name": "auth_code",
                "language": {"code": "en"},
                "components": [
                    {
                        "type": "body",
                        "parameters": [{"type": "text", "text": code}]
                    },
                    {
                        "type": "button",
                        "sub_type": "url",
                        "index": 0,
                        "parameters": [{"type": "text", "text": code}]
                    }
                ],
            }
        })
    }

    pub async fn send(&self, receipient: &str, code: &str) -> Result<(), Error> {
        let json = Self::body(receipient, code);
        self.client
            .post(URL)
            .bearer_auth(self.token)
            .json(&json)
            .send()
            .await
            .map_err(Error::server)?
            .error_for_status()
            .map_err(Error::server)?;
        Ok(())
    }
}
