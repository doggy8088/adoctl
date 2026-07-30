use std::sync::Arc;

use adoctl::{commands, credentials::KeyringCredentialStore};
use clap::Parser;

#[tokio::main]
async fn main() {
    if let Some(help) = adoctl::cli::render_help_if_missing_command(std::env::args_os().skip(1)) {
        print!("{help}");
        return;
    }

    let cli = match adoctl::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) if error.use_stderr() => {
            eprintln!("{}", adoctl::cli::render_parse_error(&error));
            std::process::exit(error.exit_code());
        }
        Err(error) => {
            print!("{error}");
            std::process::exit(error.exit_code());
        }
    };
    adoctl::debug::set_enabled(cli.debug);
    if cli.debug {
        adoctl::debug::log("已啟用 --debug，除錯資訊會輸出到 stderr。");
    }

    let store = Arc::new(KeyringCredentialStore::new());

    if let Err(error) = commands::execute(cli, store).await {
        eprintln!("{error}");
        std::process::exit(error.exit_code());
    }
}
