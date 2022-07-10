mod ninupdates;
pub mod reqwest_client;
mod swipc;

use crate::ninupdates::Region;
use app_dirs2::AppInfo;
use clap::{Parser, Subcommand};

const APP_INFO: AppInfo = AppInfo {
    name: "horizon-ipcdef-codegen",
    author: "DCNick3",
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Ninupdates(ninupdates::cli::Args),
    Swipc(swipc::cli::Args),
}

fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    match args.command {
        Command::Ninupdates(args) => ninupdates::cli::run(args),
        Command::Swipc(args) => swipc::cli::run(args),
    }
}
