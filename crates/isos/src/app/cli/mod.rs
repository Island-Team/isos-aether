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
  Run {
    /// Run the API and deployment without starting or opening the UI.
    #[arg(long)]
    no_ui: bool,
  },
  /// Serve the configuration UI and API without starting deployment workers.
  Serve {
    #[arg(long)]
    ui_live: bool,
    /// Serve only the API and manager.
    #[arg(long, conflicts_with = "ui_live")]
    no_ui: bool,
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
  /// Exercise the saved demo button and monitor through the local API.
  Demo {
    #[command(subcommand)]
    command: DemoCommand,
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
  /// Inspect backend nodes and manage hardware or service packages.
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
pub(crate) enum DemoCommand {
  /// Submit one demo button activation. Use a fresh event ID for each click.
  Activate { event_id: String },
  /// Read the outcome of a previously submitted demo activation.
  Result { event_id: String },
  /// Read the current demo arm observation.
  Monitor,
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
  /// Install a release or build a source checkout; source builds stay linked for Apply and Run refresh.
  Install(InstallArgs),
  Remove {
    id: String,
    version: String,
  },
}

#[derive(Subcommand)]
pub(crate) enum PackageConfigCommand {
  Show {
    pin: String,
  },
  Validate {
    pin: String,
    file: PathBuf,
  },
  Import {
    pin: String,
    file: PathBuf,
    #[arg(long)]
    revision: String,
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
  /// Build and verify a source candidate in an isolated store; runtime replay is required to qualify it.
  Check {
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    version: String,
    #[arg(long)]
    output: PathBuf,
    /// Candidate graph for owner catalog and interface admission.
    #[arg(long)]
    graph: Vec<PathBuf>,
    /// Deployment-local package configuration used by the isolated replay.
    #[arg(long)]
    package_config: Option<PathBuf>,
  },
  /// Publish exactly the assets from a qualified check directory.
  Release {
    #[arg(long)]
    checked: PathBuf,
  },
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
  /// Refresh available upgrades for locally installed packages only.
  Update,
  /// Rebuild source-installed packages selected by the bound graph.
  RefreshSources,
  /// Install catalog upgrades found by the last package update.
  Upgrade {
    id: Option<String>,
    /// Approval digest for one exact package asset (requires ID).
    #[arg(long)]
    approve_download: Option<String>,
  },
  Install(InstallArgs),
  /// Edit deployment-local TOML for one exact installed package pin.
  Configure {
    pin: String,
    /// Offer the selected service's verified setup helper before editing.
    #[arg(long)]
    service: Option<String>,
  },
  /// Inspect, validate, or import package configuration.
  Config {
    #[command(subcommand)]
    command: PackageConfigCommand,
  },
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
  /// Approval digest printed with the exact catalog asset preview (for scripts).
  #[arg(long)]
  approve_download: Option<String>,
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
