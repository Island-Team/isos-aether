mod commands;
pub(crate) use commands::*;
mod product;
pub(crate) use product::*;

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
  name = "isos",
  version,
  about = "Launch and manage IsOS with EI and EOS",
  after_help = "MODES\n  dev: local source and installed binaries; setup can render configuration.\n  product (default): exact verified release pins; no source build at Run.\n\nSTART\n  Run `isos` to initialize and open the local workbench.\n  Run `isos <command> --help` for command details."
)]
pub(crate) struct Cli {
  #[arg(long = "generate-completion", hide = true)]
  pub(crate) generate_completion: Option<Shell>,
  #[arg(
    long,
    global = true,
    env = "ISOS_DEPLOYMENT",
    default_value = "default"
  )]
  pub(crate) deployment: String,
  #[command(subcommand)]
  pub(crate) command: Option<CommandLine>,
}

#[derive(Subcommand)]
pub(crate) enum CommandLine {
  /// Open the browser workbench without starting EI or EOS deployment workers.
  Web,
  /// Open the terminal workbench, starting the IsOS server if needed.
  Tui,
  /// Build and install all three cores from a source checkout.
  Install { source: PathBuf },
  /// Refresh and list available unified product releases.
  Update,
  /// Install a verified unified product release.
  Upgrade { version: Option<String> },
  /// Initialize the deployment without starting processes.
  Init {
    #[arg(long, value_parser = ["dev", "product"])]
    mode: Option<String>,
  },
  /// Open the local configuration page.
  Open,
  /// Run the deployment graph.
  Run,
  /// Serve the configuration UI and API without starting deployment workers.
  Serve {
    #[arg(long)]
    ui_live: bool,
  },
  /// Manage the local Island-port account session.
  Auth {
    #[command(subcommand)]
    command: AuthCommand,
  },
  /// Start every enabled deployment node in dependency order.
  Start,
  /// Emergency stop for this workspace, independent of the manager and run ID.
  Stop {
    /// Read the configured emergency stop code from standard input.
    #[arg(long)]
    code_stdin: bool,
  },
  /// Inspect possible local processes or request an owner-controlled stop.
  PhysicalStop {
    #[command(subcommand)]
    command: PhysicalStopCommand,
  },
  /// Control only the applied deployment.
  Deployment {
    #[command(subcommand)]
    command: DeploymentCommand,
  },
  /// Stop and start the active deployment using its saved graph.
  Restart,
  /// Show deployment and per-node process status; optionally emit JSON.
  Status {
    #[arg(long)]
    json: bool,
  },
  /// Read recent unified logs for a source or all sources.
  Logs {
    #[arg(default_value = "all", env = "ISOS_LOG_DEFAULT_SOURCE")]
    source: String,
    #[arg(long, default_value_t = 100)]
    lines: u32,
    #[arg(long, default_value = "info", value_parser = ["trace", "debug", "info", "warn", "error"])]
    level: String,
    #[arg(long, default_value = "log", value_parser = ["log", "jsonl", "json"])]
    format: String,
    #[arg(long)]
    follow: bool,
    /// Read the pinned first event and current deployment-run page.
    #[arg(long)]
    current: bool,
  },
  /// Check the shared graph, package, binaries, and manager readiness.
  Doctor,
  /// Initialize, inspect, validate, or apply a deployment graph.
  Config {
    #[command(subcommand)]
    command: ConfigCommand,
  },
  /// Search, install, verify, link, or remove versioned packages.
  #[command(name = "pkg", alias = "packages")]
  Packages {
    #[command(subcommand)]
    command: PackageCommand,
  },
  /// Inspect skill nodes and manage installed skill packages.
  Skills {
    #[command(subcommand)]
    command: LibraryCommand,
  },
  /// Inspect service nodes and manage installed service packages.
  Services {
    #[command(subcommand)]
    command: ServiceCommand,
  },
  /// Inspect backend nodes and manage hardware or simulator packages.
  Backends {
    #[command(subcommand)]
    command: LibraryCommand,
  },
  /// Serve the owner-only manager API on a Unix socket.
  Daemon,
}

#[derive(Subcommand)]
pub(crate) enum DeploymentCommand {
  Stop,
  Status,
}

#[derive(Subcommand)]
pub(crate) enum PhysicalStopCommand {
  /// Inspect manager status and possible local process candidates.
  Check,
  /// Ask the IsOS manager to stop, then inspect remaining candidates.
  Stop,
  /// Record operator-reviewed safety for an interrupted historical stop.
  Recover {
    #[arg(long)]
    run_id: String,
    #[arg(long)]
    attest_safe: bool,
  },
}

#[derive(Subcommand)]
pub(crate) enum ServiceCommand {
  List,
  Validate,
  Start {
    id: String,
  },
  Stop {
    id: String,
  },
  Restart {
    id: String,
  },
  Status {
    id: Option<String>,
  },
  Logs {
    id: String,
    #[arg(long, default_value_t = 100)]
    lines: usize,
  },
  Adapter {
    #[arg(value_parser = ["models"])]
    provider: String,
    repo: PathBuf,
    alias: String,
  },
  Install(InstallArgs),
  Remove {
    id: String,
    version: String,
  },
}

#[derive(Subcommand)]
pub(crate) enum AuthCommand {
  /// Sign in with the IslandOS GitHub OAuth app.
  Login,
  /// Revoke and remove the locally saved Island-port session.
  Logout,
  /// Show the signed-in Island-port account and role.
  Whoami,
}

#[derive(Subcommand)]
pub(crate) enum ConfigCommand {
  Init {
    #[arg(long)]
    output: Option<PathBuf>,
  },
  Validate {
    file: PathBuf,
  },
  Apply {
    file: PathBuf,
    #[arg(long)]
    revision: Option<String>,
  },
  Show,
}

#[derive(Subcommand)]
pub(crate) enum PackageCommand {
  /// Upload an asset and manifest to a draft GitHub Release, then submit for review.
  Upload {
    manifest: PathBuf,
    asset: PathBuf,
  },
  Search {
    #[arg(default_value = "")]
    query: String,
  },
  Info {
    id: String,
    #[arg(long, default_value = "latest")]
    version: String,
  },
  Install(InstallArgs),
  Link {
    id: String,
    version: String,
    path: PathBuf,
    #[arg(long)]
    manifest: Option<PathBuf>,
  },
  List {
    #[arg(long)]
    json: bool,
  },
  /// List node sets contributed by installed package releases.
  NodeSets,
  /// Compose one installed node set into a graph and print the preview JSON.
  PreviewSet {
    id: String,
    version: String,
    set_id: String,
    graph: PathBuf,
    #[arg(long)]
    values: Option<PathBuf>,
  },
  Verify {
    manifest: PathBuf,
    asset: PathBuf,
  },
  /// Publish an already-uploaded GitHub release manifest to Island-port.
  Publish {
    #[arg(long)]
    manifest: PathBuf,
  },
  Remove {
    id: String,
    version: String,
  },
}

#[derive(Args)]
pub(crate) struct InstallArgs {
  id: Option<String>,
  #[arg(long, default_value = "latest")]
  version: String,
  #[arg(long)]
  manifest: Option<PathBuf>,
  #[arg(long)]
  asset: Option<PathBuf>,
  #[arg(long)]
  source: Option<PathBuf>,
  #[arg(long)]
  allow_source_build: bool,
  #[arg(long)]
  allow_unapproved: bool,
}

#[derive(Subcommand)]
pub(crate) enum LibraryCommand {
  List {
    #[arg(long)]
    json: bool,
  },
  Info {
    node: String,
  },
  Status {
    node: String,
    #[arg(long)]
    json: bool,
  },
  Install(InstallArgs),
  Remove {
    id: String,
    version: String,
  },
}
