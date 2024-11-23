use std::io;
use std::env;

mod app;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Take command-line argument for initial query (transaction hash)
    let initial_query = if args.len() > 1 {
        args[1].clone() // Use the provided argument (transaction hash)
    } else {
        eprintln!("Error: Transaction hash is required as a command-line argument.");
        std::process::exit(1);
    };

    let mut terminal = ratatui::init();
    let mut app = app::App {
        query: initial_query,
        ..app::App::default()
    };

    let app_result = app.run(&mut terminal).await;
    ratatui::restore();
    app_result
}


