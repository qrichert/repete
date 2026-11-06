mod cli;
mod runner;

use lessify::Pager;

use crate::cli::ui;

fn main() {
    let code = match cli::Config::parse(std::env::args_os().skip(1)) {
        Ok(cli::Action::Help) => {
            println!("{}\n{}", cli::help_message(), cli::longer_help_notice());
            0
        }
        Ok(cli::Action::LongHelp) => {
            Pager::page_or_print(&cli::long_help_message());
            0
        }
        Ok(cli::Action::Version) => {
            println!("{}", cli::version_message());
            0
        }
        Ok(cli::Action::Run(config)) => match runner::run(&config) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("{}: {}.", ui::Color::error("error"), error.message);
                error.code
            }
        },
        Err(message) => {
            eprintln!(
                "{error}: {message}.\nTry '{bin} -h' for help.",
                error = ui::Color::error("error"),
                bin = env!("CARGO_BIN_NAME"),
            );
            2
        }
    };
    std::process::exit(code);
}
