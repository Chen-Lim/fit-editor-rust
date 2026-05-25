use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "fit-editor",
    version,
    about = "Pure-Rust FIT file editor & viewer"
)]
pub struct Cli {
    /// Verbose output (show decode warnings)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode (results only, no decoration)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable colored output (also auto-disabled when stdout is not a TTY)
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Validate FIT file integrity (CRC, signature)
    Validate {
        /// Path to the FIT file
        file: String,
    },

    /// Show file header metadata and message counts
    Info {
        /// Path to the FIT file
        file: String,
    },

    /// Dump all messages in human-readable format
    Dump {
        /// Path to the FIT file
        file: String,

        /// Only show messages of this type (e.g. record, session, lap)
        #[arg(short, long)]
        message: Option<String>,

        /// Only show these fields (comma-separated, requires --message)
        #[arg(short, long)]
        field: Option<String>,

        /// Max messages to display
        #[arg(short, long)]
        limit: Option<usize>,

        /// Show raw values (skip scale/offset, enum conversion)
        #[arg(long)]
        raw: bool,

        /// Compact mode (one line per message)
        #[arg(long)]
        compact: bool,
    },

    /// Export messages to JSON or CSV
    Export {
        /// Path to the FIT file
        file: String,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: ExportFormat,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Only export messages of this type
        #[arg(short, long)]
        message: Option<String>,

        /// Pretty-print JSON (default: on)
        #[arg(long, default_value_t = true)]
        pretty: bool,

        /// Compact JSON
        #[arg(long)]
        compact: bool,
    },

    /// Encode JSON back to FIT file
    Encode {
        /// Path to the JSON file
        file: String,

        /// Output FIT file path
        #[arg(short, long)]
        output: String,
    },

    /// Edit fields in a FIT file
    Edit {
        /// Path to the FIT file
        file: String,

        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Set field value (field.path=value, repeatable)
        #[arg(long)]
        set: Vec<String>,

        /// Remove all messages of this type
        #[arg(long)]
        remove_message: Vec<String>,
    },

    /// Merge multiple FIT files by timestamp
    Merge {
        /// FIT files to merge
        files: Vec<String>,

        /// Output file path
        #[arg(short, long)]
        output: String,
    },

    /// Split a FIT file at a timestamp or message index
    Split {
        /// Path to the FIT file
        file: String,

        /// Output file prefix
        #[arg(short, long)]
        output: String,

        /// Split at this timestamp (RFC3339)
        #[arg(long)]
        at: Option<String>,

        /// Split at this message index
        #[arg(long)]
        at_index: Option<usize>,
    },

    /// Compare two FIT files
    Diff {
        /// First FIT file
        file1: String,

        /// Second FIT file
        file2: String,

        /// Ignore timestamp differences
        #[arg(long)]
        ignore_timestamps: bool,

        /// Only compare messages of this type
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Show activity summary statistics
    Summary {
        /// Path to the FIT file
        file: String,
    },

    /// Hex dump of FIT file contents
    Hexdump {
        /// Path to the FIT file
        file: String,

        /// Max bytes to display
        #[arg(short = 'n', long)]
        bytes: Option<usize>,

        /// Skip file header
        #[arg(long)]
        skip_header: bool,

        /// Annotate message boundaries
        #[arg(long)]
        annotate: bool,
    },

    /// Batch process multiple FIT files
    Batch {
        /// Glob pattern for input files
        glob: String,

        /// Command to run on each file
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Generate shell completion script (bash, zsh, fish, powershell, elvish)
    Completion {
        /// Target shell
        shell: Shell,
    },
}

#[derive(Clone, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
    Gpx,
}
