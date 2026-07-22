mod cli;
mod data;
mod doctor;
mod process;
mod status;
mod tui;

use std::env;

pub const APP_NAME: &str = "Pad";
pub const ENV_PREFIX: &str = "PAD";
pub const DB_FILE_NAME: &str = "notepads.json";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let cmd = args[1].to_lowercase();
        if cmd == "tui" {
            tui::run_tui();
        } else {
            cli::handle_cli_args(&args);
        }
    } else {
        tui::run_tui();
    }
}
