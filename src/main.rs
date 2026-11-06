mod cli;
mod runner;

fn main() {
    let code = match cli::Config::parse(std::env::args_os().skip(1)) {
        Ok(cli::Action::Help) => {
            println!("{}", cli::help_message());
            0
        }
        Ok(cli::Action::Version) => {
            println!("{}", cli::version_message());
            0
        }
        Ok(cli::Action::Run(config)) => match runner::run(&config) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("repete: {}", error.message);
                error.code
            }
        },
        Err(message) => {
            eprintln!("repete: {message}\nTry 'repete -h' for help.");
            2
        }
    };
    std::process::exit(code);
}
