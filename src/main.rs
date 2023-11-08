pub mod configdb;
pub mod server;
pub mod client;
pub mod edge_server;
pub mod ip_rule;
pub mod http1;
pub mod location_rule;

#[tokio::main]
async fn main() {
    println!("starting the WAF");

    loop {
        location_rule::initialize();
        edge_server::initialize();

        let thread = tokio::spawn(async move {
            let _ = server::start().await;
        });

        let _ = thread.await;
            
        // TO-DO: capture reset and restart do graceful restart
        println!("TO-DO: waiting for reset command not implemented, aborting");
        std::process::abort();
    }
}
