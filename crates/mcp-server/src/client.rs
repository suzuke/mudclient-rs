use reqwest::Client;
use serde_json::Value;

#[derive(Debug)]
pub struct MudClient {
    client: Client,
    base_url: String,
}

impl MudClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn session_param(session: &Option<String>) -> Vec<(&str, String)> {
        session.as_ref().map(|s| vec![("session", s.clone())]).unwrap_or_default()
    }

    pub async fn get(&self, path: &str, session: &Option<String>) -> Result<Value, String> {
        let resp = self.client.get(self.url(path))
            .query(&Self::session_param(session))
            .send().await
            .map_err(|e| format!("HTTP GET error: {e}"))?;
        if resp.status() == 404 {
            return Err("Session not found (404). Is the MUD client running?".into());
        }
        resp.json().await.map_err(|e| format!("JSON parse error: {e}"))
    }

    pub async fn get_with_params(&self, path: &str, params: &[(&str, String)]) -> Result<Value, String> {
        let resp = self.client.get(self.url(path))
            .query(params)
            .send().await
            .map_err(|e| format!("HTTP GET error: {e}"))?;
        if resp.status() == 404 {
            return Err("Session not found (404). Is the MUD client running?".into());
        }
        resp.json().await.map_err(|e| format!("JSON parse error: {e}"))
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let resp = self.client.post(self.url(path))
            .json(body)
            .send().await
            .map_err(|e| format!("HTTP POST error: {e}"))?;
        if resp.status() == 404 {
            return Err("Session not found (404). Is the MUD client running?".into());
        }
        resp.json().await.map_err(|e| format!("JSON parse error: {e}"))
    }

    pub async fn delete(&self, path: &str, session: &Option<String>) -> Result<Value, String> {
        let resp = self.client.delete(self.url(path))
            .query(&Self::session_param(session))
            .send().await
            .map_err(|e| format!("HTTP DELETE error: {e}"))?;
        resp.json().await.map_err(|e| format!("JSON parse error: {e}"))
    }
}
