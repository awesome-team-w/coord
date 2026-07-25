mod cli;
mod cmd;
mod db;
mod paths;
mod session;

fn main() {
    let code = match cli::run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}
