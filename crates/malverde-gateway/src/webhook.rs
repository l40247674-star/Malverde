use std::sync::Arc;

use serde::Deserialize;
use tiny_http::{Header, Response, Server};

use malverde_agent::{Agent, AgentObserver};
use malverde_core::SessionId;

use crate::platform::Platform;

#[derive(Debug, Deserialize)]
struct WebhookPayload {
    chat_id: String,
    text: String,
    #[serde(default)]
    #[allow(dead_code)]
    from: Option<String>,
}

/// Servidor webhook de loopback: expone el agente vía HTTP SOLO en 127.0.0.1.
///
/// `POST /message` con `{"chat_id": "...", "text": "..."}` → responde
/// `{"session": "...", "reply": "..."}`. Nunca acepta conexiones externas.
pub struct LoopbackWebhook {
    port: u16,
    agent: Arc<Agent>,
    observer: Arc<dyn AgentObserver>,
}

impl LoopbackWebhook {
    pub fn new(port: u16, agent: Arc<Agent>, observer: Arc<dyn AgentObserver>) -> Self {
        Self { port, agent, observer }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Arranca el servidor (bloqueante). Solo acepta conexiones de loopback.
    pub fn serve(&self) -> malverde_core::Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Server::http(&addr)
            .map_err(|e| malverde_core::MalverdeError::gateway(format!("no se pudo abrir {addr}: {e}")))?;
        tracing::info!("gateway webhook escuchando en http://{addr} (solo loopback)");

        for request in server.incoming_requests() {
            let response = self.handle_request(request);
            if let Err(e) = response {
                tracing::error!("error respondiendo petición: {e}");
            }
        }
        Ok(())
    }

    fn handle_request(&self, mut request: tiny_http::Request) -> malverde_core::Result<()> {
        let (status, body) = match (request.method(), request.url()) {
            (tiny_http::Method::Get, "/health") => (200, serde_json::json!({"status": "ok"}).to_string()),
            (tiny_http::Method::Post, "/message") => {
                let mut body = String::new();
                let read = request.as_reader().read_to_string(&mut body).map_err(malverde_core::MalverdeError::io);
                match read.and_then(|_| serde_json::from_str::<WebhookPayload>(&body).map_err(|e| malverde_core::MalverdeError::gateway(format!("payload inválido: {e}")))) {
                    Ok(payload) => match self.dispatch(&payload) {
                        Ok(reply) => (200, serde_json::json!({"reply": reply}).to_string()),
                        Err(e) => (500, serde_json::json!({"error": e.to_string()}).to_string()),
                    },
                    Err(e) => (400, serde_json::json!({"error": e.to_string()}).to_string()),
                }
            }
            _ => (404, serde_json::json!({"error": "ruta no encontrada; usa POST /message"}).to_string()),
        };

        let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
        let response = Response::from_string(body).with_status_code(status).with_header(header);
        request.respond(response).map_err(|e| malverde_core::MalverdeError::gateway(e.to_string()))
    }

    fn dispatch(&self, payload: &WebhookPayload) -> malverde_core::Result<String> {
        let session = SessionId::from_raw(format!("webhook-{}", payload.chat_id));
        let result = self.agent.run(&session, &payload.text, self.observer.as_ref())?;
        Ok(result.final_text)
    }
}

impl Platform for LoopbackWebhook {
    fn name(&self) -> &str {
        "webhook"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_agent::{Agent, AgentConfig, AgentObserver};
    use std::io::Read as _;
    use malverde_memory::MemoryStore;
    use malverde_provider::MockProvider;
    use malverde_tools::{PermissionManager, ToolRegistry};
    use std::io::Write;
    use std::net::TcpStream;

    struct Silent;
    impl AgentObserver for Silent {}

    fn spawn_agent(_port: u16) -> Arc<Agent> {
        let provider: Arc<dyn malverde_provider::LlmProvider> = Arc::new(MockProvider::new("mock"));
        let memory = Arc::new(MemoryStore::open_in_memory().unwrap());
        let mut agent = Agent::new(
            provider,
            ToolRegistry::with_builtins(),
            memory,
            malverde_skills_support(),
            AgentConfig { save_traces: false, ..Default::default() },
            PermissionManager::new(malverde_core::PermissionMode::AutoApprove, &[]),
        );
        agent.install_ext_tools();
        Arc::new(agent)
    }

    fn malverde_skills_support() -> malverde_skills::SkillRegistry {
        malverde_skills::SkillRegistry::default()
    }

    #[test]
    fn webhook_health_and_message() {
        // Puerto efímero: pedir al SO uno libre.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let agent = spawn_agent(port);
        let gw = LoopbackWebhook::new(port, agent, Arc::new(Silent));
        let handle = std::thread::spawn(move || gw.serve().unwrap());
        std::thread::sleep(std::time::Duration::from_millis(300));

        let mut conn = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let req = "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
        conn.write_all(req.as_bytes()).unwrap();
        let mut buf = String::new();
        let _ = conn.read_to_string(&mut buf);
        assert!(buf.contains("200"));
        assert!(buf.contains("ok"));

        // POST /message
        let mut conn = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let body = serde_json::json!({"chat_id": "chat1", "text": "hola"}).to_string();
        let req = format!(
            "POST /message HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        conn.write_all(req.as_bytes()).unwrap();
        let mut buf = String::new();
        let _ = conn.read_to_string(&mut buf);
        assert!(buf.contains("reply"));

        // El servidor queda en el hilo; terminamos el test.
        drop(handle);
    }
}
