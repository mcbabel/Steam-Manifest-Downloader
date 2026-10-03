mod app;
mod cli;
mod i18n;
mod term;
mod theme;
mod ui;

use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "smd",
    version = app::VERSION,
    about = "Steam Manifest Downloader — terminal UI and headless CLI",
    long_about = "Without a command, opens the interactive terminal UI (mouse and keyboard).\n\
                  The commands below run without a UI, for servers, containers and scripts.\n\
                  Settings, history and caches are shared with the desktop app."
)]
struct Cli {
    #[arg(long, global = true, env = "SMD_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[arg(long, short, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Tui,
    Download(cli::DownloadArgs),
    Search(cli::SearchArgs),
    History(cli::HistoryArgs),
}

fn main() {
    let args = Cli::parse();
    let data_dir = args
        .data_dir
        .unwrap_or_else(smd_core::paths::default_app_data_dir);
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to start the async runtime: {}", e);
            std::process::exit(1);
        }
    };

    let code = match args.command {
        None | Some(Command::Tui) => match runtime.block_on(run_tui(data_dir)) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: {}", e);
                1
            }
        },
        Some(cmd) => {
            if !args.verbose {
                term::redirect_stderr();
            }
            let code = runtime.block_on(async move {
                match cmd {
                    Command::Download(a) => cli::download(data_dir, a).await,
                    Command::Search(a) => cli::search(data_dir, a).await,
                    Command::History(a) => cli::history(data_dir, a).await,
                    Command::Tui => unreachable!(),
                }
            });
            code
        }
    };
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    std::process::exit(code);
}

async fn run_tui(data_dir: PathBuf) -> std::io::Result<()> {
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        return Err(std::io::Error::other(
            "the terminal UI needs an interactive terminal — use `smd download` / `smd search` in scripts (see `smd --help`)",
        ));
    }
    let (app, rx) = app::App::new(data_dir).await;
    let mut terminal = term::init()?;
    let result = app.run(&mut terminal, rx).await;
    term::restore();
    result
}
