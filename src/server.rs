use crate::configdb;
use crate::client;

pub async fn start() {
    let general_config = match std::fs::read_to_string(configdb::GENERAL_CONFIG_FILENAME) {
        Ok(content) => {
            match serde_yaml::from_str::<configdb::ConfigGeneral>(content.as_str()) {
                Ok(object) => {
                    object
                },
                Err(err) => {
                    eprintln!("failed to deserialize the file {}, error: {}", configdb::GENERAL_CONFIG_FILENAME, err.to_string());
                    return;
                }
            }
        },
        Err(err) => {
            eprintln!("failed to read from {}, error: {}", configdb::GENERAL_CONFIG_FILENAME, err.to_string());
            return;
        }
    };

    let listener = match tokio::net::TcpListener::bind(format!("{}:{}", general_config.listen_address, general_config.listen_port)).await {
        Ok(listener) => { listener },
        Err(err) => {
            eprintln!("failed to bind the address {}:{}, error: {}", general_config.listen_address, general_config.listen_port, err.to_string());
            return;
        }
    };

    let conn_list: std::sync::Arc<std::sync::Mutex<(usize, Vec<tokio::task::JoinHandle<()>>)>> = std::sync::Arc::new(std::sync::Mutex::new((0, Vec::new())));

    let conn_list_cleaner_param = std::sync::Arc::clone(&conn_list);
    let conn_list_cleaner = tokio::spawn(async move {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            match conn_list_cleaner_param.lock() {
                Ok(mut locked_value) => {
                    let mut for_remove: Vec<usize> = Vec::new();

                    for (idx, value) in locked_value.1.iter().enumerate() {
                        if value.is_finished() {
                            for_remove.push(idx);
                        }
                    }

                    for idx in for_remove.iter().rev() {
                        locked_value.0 = locked_value.0 - 1; // decrement the number of connections
                        locked_value.1.remove(*idx);
                    }
                },
                Err(err) => {
                    eprintln!("internal error, failed to lock the variable 'conn_list', error: {}; aborting!", err.to_string());
                    std::process::abort();
                }
            }
        }
    });

    loop {
        let conn_tuple = match listener.accept().await {
            Ok(conn) => { conn },
            Err(err) => {
                eprintln!("failed to accept a client, error: {}", err.to_string());
                break;
            }
        };

        let conn = conn_tuple.0;
        let connaddr = conn_tuple.1;

        match std::sync::Arc::clone(&conn_list).lock() {
            Ok(mut locked_value) => {
                if locked_value.0 >= general_config.maximum_connections { // check the number of connections if it reaches the limit
                    println!("refusing to accept {} due limit of number of connections reached", connaddr.to_string());
                    continue;
                }

                locked_value.0 = locked_value.0 + 1; // increment the number of connections
                locked_value.1.push(tokio::spawn(async move { client::handler(conn, connaddr).await }));
            },
            Err(err) => {
                eprintln!("internal error, failed to lock the variable 'conn_list', error: {}; aborting!", err.to_string());
                std::process::abort();
            }
        }
    }

    conn_list_cleaner.abort();
}
