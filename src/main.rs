pub mod configdb;
pub mod server;
pub mod client;
pub mod edge_server;
pub mod ip_rule;

#[tokio::main]
async fn main() {
    println!("starting the WAF");

    loop {
        match edge_server::initialize() {
            Ok(_) => {
                let thread = tokio::spawn(async move {
                    let _ = server::start().await;
                });
        
                let _ = thread.await;
            },
            Err(err) => {
                eprintln!("failed to initialize the edge server module, error: {}", err.to_string());
            }
        }

        // TO-DO: capture reset and restart do graceful restart
        println!("TO-DO: waiting for reset command not implemented, aborting");
        std::process::abort();
    }
}
