mod app;
mod cli;
mod i18n;
mod term;
mod theme;
mod ui;

use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "smd",
    version = app::VERSION,
    about = i18n::t("tui.cli.about"),
    long_about = i18n::t("tui.cli.longAbout")
)]
struct Cli {
    #[arg(
        long,
        global = true,
        env = "SMD_DATA_DIR",
        help = i18n::t("tui.cli.argDataDir")
    )]
    data_dir: Option<PathBuf>,

    #[arg(
        long,
        short,
        global = true,
        help = i18n::t("tui.cli.argVerbose")
    )]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = i18n::t("tui.cli.cmdTui"))]
    Tui,
    #[command(about = i18n::t("tui.cli.cmdDownload"))]
    Download(cli::DownloadArgs),
    #[command(about = i18n::t("tui.cli.cmdSearch"))]
    Search(cli::SearchArgs),
    #[command(about = i18n::t("tui.cli.cmdHistory"))]
    History(cli::HistoryArgs),
    #[command(about = i18n::t("tui.cli.cmdTelemetry"))]
    Telemetry(cli::TelemetryArgs),
}

fn main() {
    i18n::init_from_data_dir(&smd_core::paths::default_app_data_dir());
    let matches = localized_command().get_matches();
    let args = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    let data_dir = args
        .data_dir
        .unwrap_or_else(smd_core::paths::default_app_data_dir);
    i18n::init_from_data_dir(&data_dir);
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "{}",
                i18n::tf(
                    "tui.cli.error",
                    &[(
                        "message",
                        &i18n::tf("tui.cli.runtimeFailed", &[("message", &e)])
                    )]
                )
            );
            std::process::exit(1);
        }
    };

    let interactive = std::io::stdout().is_terminal() && std::io::stdin().is_terminal();
    let code = match args.command {
        None if !interactive => {
            let _ = localized_command().print_help();
            0
        }
        None | Some(Command::Tui) => match runtime.block_on(run_tui(data_dir)) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{}", i18n::tf("tui.cli.error", &[("message", &e)]));
                1
            }
        },
        Some(cmd) => {
            if !args.verbose {
                term::redirect_stderr();
            }
            let code = runtime.block_on(async move {
                cli::listen_for_shutdown();
                if let Command::Telemetry(a) = cmd {
                    return cli::telemetry(data_dir, a).await;
                }
                let (name, props) = match &cmd {
                    Command::Download(a) => (
                        "download",
                        serde_json::json!({
                            "json": a.json, "update": a.update.is_some(), "repair": a.repair,
                            "like_steam": a.like_steam, "all_depots": a.all_depots, "dlc": a.dlc,
                            "shutdown": a.shutdown, "list": a.list, "speed_limit": a.speed_limit.is_some(),
                            "depots_given": !a.depots.is_empty(), "out": a.out.is_some(),
                        }),
                    ),
                    Command::Search(a) => ("search", serde_json::json!({ "json": a.json, "manifests": a.manifests.is_some() })),
                    Command::History(a) => ("history", serde_json::json!({ "json": a.json })),
                    _ => ("other", serde_json::json!({})),
                };
                cli::telemetry_start(&data_dir, name, props).await;
                let code = match cmd {
                    Command::Download(a) => cli::download(data_dir, a).await,
                    Command::Search(a) => tokio::select! {
                        code = cli::search(data_dir, a) => code,
                        _ = cli::stopped() => 130,
                    },
                    Command::History(a) => tokio::select! {
                        code = cli::history(data_dir, a) => code,
                        _ = cli::stopped() => 130,
                    },
                    Command::Tui | Command::Telemetry(_) => unreachable!(),
                };
                cli::telemetry_finish().await;
                code
            });
            code
        }
    };
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    std::process::exit(code);
}

#[derive(Clone, Copy)]
struct Headings {
    commands: &'static str,
    options: &'static str,
    arguments: &'static str,
}

fn localized_command() -> clap::Command {
    let leak = |key: &str| -> &'static str { Box::leak(i18n::t(key).into_boxed_str()) };
    let headings = Headings {
        commands: leak("tui.cli.headCommands"),
        options: leak("tui.cli.headOptions"),
        arguments: leak("tui.cli.headArguments"),
    };
    let mut cmd = Cli::command();
    cmd.build();
    localize_command(cmd, headings)
}

fn localize_command(cmd: clap::Command, headings: Headings) -> clap::Command {
    let style = cmd.get_styles().get_usage();
    let template = format!(
        "{{before-help}}{{about-with-newline}}\n{}{}{} {{usage}}\n\n{{all-args}}{{after-help}}",
        style.render(),
        i18n::t("tui.cli.headUsage"),
        style.render_reset()
    );
    let is_help = cmd.get_name() == "help";
    let mut cmd = cmd
        .help_template(template)
        .subcommand_help_heading(headings.commands)
        .mut_args(|a| {
            let heading = if a.is_positional() {
                headings.arguments
            } else {
                headings.options
            };
            let a = a.help_heading(heading);
            match a.get_id().as_str() {
                "help" => a
                    .help(i18n::t("tui.cli.printHelp"))
                    .long_help(i18n::t("tui.cli.printHelpLong")),
                "version" => a.help(i18n::t("tui.cli.printVersion")),
                "subcommand" if is_help => a.help(i18n::t("tui.cli.helpFor")),
                _ => a,
            }
        });
    if is_help {
        cmd = cmd.about(i18n::t("tui.cli.helpCommand"));
    }
    cmd.mut_subcommands(|c| localize_command(c, headings))
}

async fn run_tui(data_dir: PathBuf) -> std::io::Result<()> {
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        return Err(std::io::Error::other(i18n::t("tui.cli.noTerminal")));
    }
    let (app, rx) = app::App::new(data_dir).await;
    let mut terminal = term::init()?;
    let result = app.run(&mut terminal, rx).await;
    term::restore();
    result
}
