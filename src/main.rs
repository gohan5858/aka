use aka::run_cli;

#[tokio::main]
async fn main() {
    if let Err(e) = run_cli().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
