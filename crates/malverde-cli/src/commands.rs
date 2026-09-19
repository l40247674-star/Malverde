use malverde_core::{ProjectId, ActorId, JobId};

/// Helper functions for CLI
pub fn print_header(title: &str) {
    println!("
{}", "=".repeat(50));
    println!("{}", title);
    println!("{}", "=".repeat(50));
}

pub fn print_success(message: &str) {
    println!("[OK] {}", message);
}

pub fn print_error(message: &str) {
    println!("[ERROR] {}", message);
}

pub fn print_info(message: &str) {
    println!("[INFO] {}", message);
}
