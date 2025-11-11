use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "certgen")]
#[command(author, version, about, long_about = None)]
#[command(about = "Generate certificates from ODF templates")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Fill a single certificate
    Fill {
        /// Template file path
        #[arg(short, long)]
        template: String,

        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Name for the certificate
        #[arg(short, long)]
        name: String,

        /// Course/Event title
        #[arg(short = 'T', long)]
        title: String,

        /// Date for single-day courses (or end date for multi-day)
        #[arg(short, long)]
        date: String,

        /// Start date for multi-day courses (optional)
        #[arg(long)]
        date_from: Option<String>,

        /// End date for multi-day courses (optional)
        #[arg(long)]
        date_to: Option<String>,

        /// Agenda/Course content
        #[arg(short, long)]
        agenda: String,

        /// Additional custom fields in format KEY=VALUE (can be used multiple times)
        #[arg(short = 'f', long = "field", value_parser = parse_key_val)]
        custom_fields: Vec<(String, String)>,
    },

    /// Fill certificates from JSON file
    Batch {
        /// Template file path
        #[arg(short, long)]
        template: String,

        /// JSON file with certificate data
        #[arg(short, long)]
        json: String,

        /// Output directory
        #[arg(short, long, default_value = "output")]
        output_dir: String,
    },

    /// Generate example JSON file
    Example {
        /// Output path for example JSON
        #[arg(short, long, default_value = "example.json")]
        output: String,

        /// Include extended fields in example
        #[arg(short = 'x', long)]
        extended: bool,
    },

    /// Interactively create a JSON file with certificate data
    CreateJson {
        /// Output path for JSON file
        #[arg(short, long, default_value = "teilnehmer.json")]
        output: String,
    },
}

/// Parse a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}
