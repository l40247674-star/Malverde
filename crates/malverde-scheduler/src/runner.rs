use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use malverde_agent::Agent;
use malverde_agent::AgentObserver;
use malverde_core::SessionId;
use tracing::info;

use crate::jobs::CronStore;

/// Daemon que revisa los trabajos cron y ejecuta sus prompts con el agente.
pub struct Scheduler {
    store: CronStore,
    agent: Arc<Agent>,
    observer: Arc<dyn AgentObserver>,
    tick: Duration,
}

impl Scheduler {
    pub fn new(store: CronStore, agent: Arc<Agent>, observer: Arc<dyn AgentObserver>) -> Self {
        Self { store, agent, observer, tick: Duration::from_secs(30) }
    }

    pub fn with_tick(mut self, tick: Duration) -> Self {
        self.tick = tick;
        self
    }

    /// Bloquea ejecutando el bucle del planificador (requiere runtime tokio).
    pub async fn run_forever(&self) -> malverde_core::Result<()> {
        info!("planificador iniciado");
        loop {
            self.tick_once().await?;
            tokio::time::sleep(self.tick).await;
        }
    }

    /// Una pasada: ejecuta los trabajos cuyo momento ha llegado.
    pub async fn tick_once(&self) -> malverde_core::Result<usize> {
        let now = Utc::now();
        let mut executed = 0;
        for job in self.store.list()? {
            if !job.enabled {
                continue;
            }
            let cron = crate::cron::Cron::parse(&job.cron)?;
            let reference = job.last_run.unwrap_or(now - chrono::Duration::seconds(1));
            if cron.matches(now) && now > reference {
                info!(job = %job.name, "ejecutando trabajo programado");
                let sid = SessionId::from_raw(format!("cron-{}", job.id));
                let agent = Arc::clone(&self.agent);
                let observer = Arc::clone(&self.observer);
                let prompt = job.prompt.clone();
                let result =
                    tokio::task::spawn_blocking(move || agent.run(&sid, &prompt, observer.as_ref())).await;
                executed += 1;
                self.store.mark_run(&job.id, now)?;
                match result {
                    Ok(Ok(res)) => info!(job = %job.name, stop = ?res.stop_reason, "trabajo terminado"),
                    Ok(Err(e)) => tracing::error!(job = %job.name, error = %e, "trabajo falló"),
                    Err(e) => tracing::error!(job = %job.name, error = %e, "trabajo falló (join)"),
                }
            }
        }
        Ok(executed)
    }
}
