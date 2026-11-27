//! Malverde Scheduler
//!
//! Cron en formato estándar (min hora dom mes dow) para ejecutar prompts del
//! agente de forma desatendida, con persistencia en JSON y daemon de ejecución.

pub mod cron;
pub mod jobs;
pub mod runner;

pub use cron::Cron;
pub use jobs::{CronJob, CronStore};
pub use runner::Scheduler;
