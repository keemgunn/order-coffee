//! Configuration and CLI argument handling

use clap::Parser;

/// CLI argument parsing structure
#[derive(Parser)]
#[command(name = "order-coffee")]
#[command(about = "A state-managed HTTP server to control system suspension")]
#[command(version = "2.0.0")]
pub struct Config {
    /// Port to bind the server to
    #[arg(short, long, default_value = "20553")]
    pub port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Suspension timer duration in minutes
    #[arg(short, long, default_value = "10")]
    pub timer: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

impl Config {
    /// Parse configuration from command line arguments
    pub fn parse() -> Self {
        Parser::parse()
    }

    /// Get the server address as a formatted string
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the appropriate log level based on verbose flag
    pub fn log_level(&self) -> &'static str {
        if self.verbose { "debug" } else { "info" }
    }
}
