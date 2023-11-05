use crate::configdb;
use crate::edge_server;
use crate::server;

async fn connect_to_https_edge_server<Address: AsRef<str> + tokio::net::ToSocketAddrs + std::fmt::Display>(address: Address, resolved_name: &str) -> Result<server::TcpClient, std::io::Error> {
    match openssl::ssl::SslConnector::builder(openssl::ssl::SslMethod::tls()) {
        Ok(mut ssl_builder) => {
            ssl_builder.set_verify(openssl::ssl::SslVerifyMode::NONE); // accept self-signed certificates

            let ssl_connector = ssl_builder.build();

            match ssl_connector.configure() {
                Ok(ssl_config) => {
                    match ssl_config.into_ssl(resolved_name) {
                        Ok(ssl) => {
                            match tokio::net::TcpStream::connect(&address).await {
                                Ok(conn) => {
                                    match tokio_openssl::SslStream::new(ssl, conn) {
                                        Ok(mut conn_ssl) => {
                                            match tokio_openssl::SslStream::connect(std::pin::Pin::new(&mut conn_ssl)).await {
                                                Ok(_) => {
                                                    return Ok(server::TcpClient::Https(conn_ssl));
                                                },
                                                Err(err) => {
                                                    eprintln!("SSL error from {}, error: {}", address, err.to_string());
                                                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
                                                }
                                            };
                                        },
                                        Err(err) => {
                                            eprintln!("SSL error from {}, error: {}", address, err.to_string());
                                            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
                                        }
                                    };
                                },
                                Err(err) => {
                                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
                                }
                            };
                        },
                        Err(err) => {
                            eprintln!("SSL error from {}, error: {}", address, err.to_string());
                            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
                        }
                    };
                },
                Err(err) => {
                    eprintln!("SSL error from {}, error: {}", address, err.to_string());
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
                }
            };

        },
        Err(err) => {
            eprintln!("SSL error from {}, error: {}", address, err.to_string());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
        }
    };
}

async fn connect_to_http_edge_server<Address: AsRef<str> + tokio::net::ToSocketAddrs>(address: Address) -> Result<server::TcpClient, std::io::Error> {
    match tokio::net::TcpStream::connect(address).await {
        Ok(conn) => {
            return Ok(server::TcpClient::Http(conn));
        },
        Err(err) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
        }
    };
}

async fn procedure(mut conn: server::TcpClient, connaddr: String, edge_info: &configdb::ConfigEdge) {
    let edgeaddr = format!("{}:{}", edge_info.destination, edge_info.destination_port);

    let mut edge_conn = match edge_info.https {
        true => {
            let result = match connect_to_https_edge_server(&edgeaddr, &edge_info.resolve_name).await {
                Ok(edge_conn) => {
                    edge_conn
                },
                Err(err) => {
                    eprintln!("failed to connect to edge server {}, error: {}", &edgeaddr, err.to_string());
                    return;
                }
            };

            result
        },
        false => {
            let result = match connect_to_http_edge_server(&edgeaddr).await {
                Ok(edge_conn) => {
                    edge_conn
                },
                Err(err) => {
                    eprintln!("failed to connect to edge server {}, error: {}", &edgeaddr, err.to_string());
                    return;
                }
            };

            result
        }
    };

    let mut conn_mtu_block = [0 as u8; 1500];
    let mut edge_mtu_block = [0 as u8; 1500];

    loop {
        tokio::select! {
            conn_read = conn.read(&mut conn_mtu_block) => {
                match conn_read {
                    Ok(0) => {
                        eprintln!("client {} closed the connection", &connaddr);
                        return;
                    },
                    Ok(len) => {
                        match edge_conn.write(&conn_mtu_block[..len]).await {
                            Ok(_) => { },
                            Err(err) => {
                                eprintln!("failed to move data from client {} to edge server {}, error: {}; closing the connection", &connaddr, &edgeaddr, err.to_string());
                                return;
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("failed to read from {}, error: {}; closing the connection", &connaddr, err.to_string());
                        return;
                    }
                }
            }
            edge_read = edge_conn.read(&mut edge_mtu_block) => {
                match edge_read {
                    Ok(0) => {
                        eprintln!("edge {} closed the connection", &edgeaddr);
                        return;
                    },
                    Ok(len) => {
                        match conn.write(&edge_mtu_block[..len]).await {
                            Ok(_) => { },
                            Err(err) => {
                                eprintln!("failed to move data from edge {} to client {}, error: {}; closing the connection", &edgeaddr, &connaddr, err.to_string());
                                return;
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("failed to read from {}, error: {}; closing the connection", &edgeaddr, err.to_string());
                        return;
                    }
                }
            }
        }
    }
}

pub async fn handler(conn: server::TcpClient, connaddr: std::net::SocketAddr) {
    let _ = conn;
    let connaddr_friendly = connaddr.to_string();

    println!("new connection {connaddr_friendly}");

    match edge_server::find_edge_server() {
        Ok(edge_info) => {
            procedure(conn, connaddr_friendly.clone(), &edge_info).await;
            edge_server::decrement_conn_count(edge_info.destination);
            println!("the connection with {}, closed", connaddr_friendly.clone());
        },
        Err(err) => {
            eprintln!("failed to find an edge server, error: {}, dropping the connection", err.to_string());
        }
    }
}
