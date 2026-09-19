use clap::{Parser, Subcommand};
use malverde_core::{ProjectId, ActorId};

/// Malverde Core Framework CLI
#[derive(Debug, Parser)]
#[command(name = "malverde")]
#[command(author = "wero")]
#[command(version = "0.1.0")]
#[command(about = "Command-line interface for Malverde Core Framework")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init { name: String },
    Start,
    Stop,
    Project(ProjectCommands),
    Actor(ActorCommands),
    Job(JobCommands),
    Info,
    Health,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommands {
    Create { name: String },
    List,
    Show { id: ProjectId },
}

#[derive(Debug, Subcommand)]
pub enum ActorCommands {
    Create { name: String, r#type: String },
    List,
    Show { id: ActorId },
}

#[derive(Debug, Subcommand)]
pub enum JobCommands {
    Create { project_id: ProjectId, name: String },
    List,
    Show { id: String },
    Start { id: String },
}

fn main() {
    let cli = Cli::parse();
    println!("Malverde CLI v0.1.0");
    match cli.command {
        Commands::Init { name } => println!("Initializing project: {}", name),
        Commands::Start => println!("Starting Malverde..."),
        Commands::Stop => println!("Stopping Malverde..."),
        Commands::Project(sub) => match sub {
            ProjectCommands::Create { name } => println!("Creating project: {}", name),
            ProjectCommands::List => println!("Listing projects..."),
            ProjectCommands::Show { id } => println!("Showing project: {}", id),
        },
        Commands::Actor(sub) => match sub {
            ActorCommands::Create { name, r#type } => println!("Creating actor: {} ({})", name, r#type),
            ActorCommands::List => println!("Listing actors..."),
            ActorCommands::Show { id } => println!("Showing actor: {}", id),
        },
        Commands::Job(sub) => match sub {
            JobCommands::Create { project_id, name } => println!("Creating job: {}", name),
            JobCommands::List => println!("Listing jobs..."),
            JobCommands::Show { id } => println!("Showing job: {}", id),
            JobCommands::Start { id } => println!("Starting job: {}", id),
        },
        Commands::Info => println!("Malverde Core Framework v0.1.0"),
        Commands::Health => println!("System health: OK"),
    }
}
