use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use crate::configdb;
use crate::client;

pub enum TcpClient {
    Http(tokio::net::TcpStream),
    Https(tokio_openssl::SslStream<tokio::net::TcpStream>),
}

impl TcpClient {
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            TcpClient::Http(http) => {
                match http.read(buf).await {
                    Ok(len) => {
                        return Ok(len);
                    },
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
            TcpClient::Https(https) => {
                match https.read(buf).await {
                    Ok(len) => {
                        return Ok(len);
                    },
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        };
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match self {
            TcpClient::Http(http) => {
                match http.write(buf).await {
                    Ok(len) => {
                        return Ok(len);
                    },
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
            TcpClient::Https(https) => {
                match https.write(buf).await {
                    Ok(len) => {
                        return Ok(len);
                    },
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        };
    }
}

fn create_ssl_server(ssl_cert: String, ssl_key: String) -> Result<openssl::ssl::SslAcceptor, std::io::Error> {
    match openssl::ssl::SslAcceptor::mozilla_intermediate(openssl::ssl::SslMethod::tls_server()) {
        Ok(mut ssl_accepter) => {
            if let Err(err) = ssl_accepter.set_private_key_file(ssl_key, openssl::ssl::SslFiletype::PEM) {
                eprintln!("SSL error: {}", err.to_string());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));    
            }

            if let Err(err) = ssl_accepter.set_certificate_chain_file(ssl_cert) {
                eprintln!("SSL error: {}", err.to_string());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));    
            }

            if let Err(err) = ssl_accepter.check_private_key() {
                eprintln!("SSL error: {}", err.to_string());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));    
            }

            return Ok(ssl_accepter.build());
        },
        Err(err) => {
            eprintln!("SSL error: {}", err.to_string());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
        }
    }
}

async fn create_http_server(address: String) -> Result<tokio::net::TcpListener, std::io::Error> {
    match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => {
            return Ok(listener);
        },
        Err(err) => {
            eprintln!("failed to bind the address {}, error: {}", address, err.to_string());
            return Err(err);
        }
    };
}

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

    let listener = match create_http_server(format!("{}:{}", general_config.listen_address, general_config.listen_port)).await {
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
        let listener_ssl: Option<openssl::ssl::SslAcceptor> = match general_config.https {
            true => {
                match create_ssl_server(general_config.ssl_certificate.clone(), general_config.ssl_certificate_key.clone()) {
                    Ok(ssl_accepter) => {
                        Some(ssl_accepter)
                    },
                    Err(err) => {
                        eprintln!("failed to create a SSL layer, error: {}", err.to_string());
                        return;
                    }
                }
            },
            false => {
                None
            }
        };

        let conn_tuple: Option<(TcpClient, std::net::SocketAddr)> = match listener_ssl {
            Some(listener_ssl) => {
                match listener.accept().await {
                    Ok(conn) => {
                        match openssl::ssl::Ssl::new(listener_ssl.clone().context()) {
                            Ok(ssl) => {
                                match tokio_openssl::SslStream::new(ssl, conn.0) {
                                    Ok(mut ssl_stream) => {
                                        match tokio_openssl::SslStream::accept(std::pin::Pin::new(&mut ssl_stream)).await {
                                            Ok(_) => {
                                                Some((TcpClient::Https(ssl_stream), conn.1))
                                            },
                                            Err(err) => {
                                                eprintln!("SSL error: {}", err.to_string());
                                                None
                                            }        
                                        }
                                    },
                                    Err(err) => {
                                        eprintln!("SSL error: {}", err.to_string());
                                        break;
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("SSL error: {}", err.to_string());
                                break;
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("failed to accept a client, error: {}", err.to_string());
                        break;
                    }
                }  
            },
            None => {
                match listener.accept().await {
                    Ok(conn) => {
                        Some((TcpClient::Http(conn.0), conn.1))
                    },
                    Err(err) => {
                        eprintln!("failed to accept a client, error: {}", err.to_string());
                        break;
                    }
                }    
            }
        };

        if let Some(conn_tuple) = conn_tuple {
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
    }

    conn_list_cleaner.abort();
}
