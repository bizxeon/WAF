use crate::configdb;
use crate::edge_server;
use crate::location_rule;
use crate::server;
use crate::ip_rule;
use crate::http1;

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

async fn procedure(mut conn: server::TcpClient, connaddr: String, edge_info: &configdb::Edge, ip_rule: &Option<configdb::IpRule>, general_config: &configdb::General) {
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
    let mut conn_request_storage: Vec<u8> = Vec::new();
    let mut conn_request_body_size: usize = 0;
    let mut conn_request_idx: usize = 0;
    let mut conn_request_body_state = false;
    let mut conn_request_bypass = false;
    const CONN_REQUEST_STORAGE_HARD_LIMIT: usize = 128 * 1024;

    loop {
        tokio::select! {
            conn_read = conn.read(&mut conn_mtu_block) => {
                match conn_read {
                    Ok(0) => {
                        println!("client {} closed the connection", &connaddr);
                        return;
                    },
                    Ok(len) => {
                        if conn_request_body_state == false {
                            if conn_request_storage.len() + len > CONN_REQUEST_STORAGE_HARD_LIMIT {
                                println!("hard limit on request header reached, dropping connection with {}", &connaddr);
                                return;
                            } 

                            conn_request_storage.append(&mut conn_mtu_block[..len].to_vec());

                            let header_length = conn_request_storage.windows(4).position(|a| a == b"\r\n\r\n");

                            if let Some(header_length) = header_length { 
                                match http1::parse(conn_request_storage[..header_length].to_vec()) {                                    
                                    Ok(object) => {
                                        if let Some(content_length) = object.properties.get("Content-Length") {
                                            match content_length.parse() {
                                                Ok(content_length) => {
                                                    conn_request_body_state = true;
                                                    conn_request_body_size = content_length;

                                                    if (header_length + 4) < len {
                                                        conn_request_idx = len - (header_length + 4);
                                                    }
                                                },
                                                Err(err) => {
                                                    eprintln!("corrupted request from client {}, error: {}; dropping the connection", &connaddr, err.to_string());
                                                    return;
                                                }
                                            }
                                        }

                                        if let Some(ip_rule) = ip_rule {
                                            if ip_rule.blacklisted_locations.contains(&object.location) {
                                                println!("dropping connection with {}, blocked by rule", &connaddr);
                                                return;
                                            }

                                            if !ip_rule.whitelist_location.contains(&object.location) {
                                                println!("dropping connection with {}, blocked by rule", &connaddr);
                                                return;
                                            }
                                        }

                                        if let Some(location_rule) = location_rule::get_location_rule(object.method, object.location) {
                                            if location_rule.bypass == true { 
                                                conn_request_bypass = true;
                                            }

                                            match location_rule.ingress {
                                                configdb::RuleGress::GenericRule => {
                                                    if matches!(general_config.ingress, configdb::GenericRuleGress::Deny) {
                                                        println!("dropping connection with {}, blocked by rule", &connaddr);
                                                        return;
                                                    }
                                                },
                                                configdb::RuleGress::Deny => {
                                                    println!("dropping connection with {}, blocked by rule", &connaddr);
                                                    return;
                                                },
                                                _ => {}
                                            }
                                        }
                                    },
                                    Err(err) => {
                                        eprintln!("processing the request from {} failed, error: {}", &connaddr, err.to_string());
                                        return;
                                    }
                                }

                                conn_request_storage.clear();
                            }
                        } else {
                            conn_request_idx = conn_request_idx + len;

                            if conn_request_bypass {
                                // TODO
                            } else {
                                // TODO
                            }

                            if conn_request_idx >= conn_request_body_size {
                                conn_request_body_state = false;
                                conn_request_body_size = 0;
                                conn_request_idx = 0;
                                conn_request_bypass = false;
                            }
                        }

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
                        println!("edge {} closed the connection", &edgeaddr);
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

pub async fn handler(conn: server::TcpClient, connaddr: std::net::SocketAddr, general_config: configdb::General) {
    let connaddr_friendly = connaddr.to_string();
    let ip_rule = ip_rule::get_ip_rule(connaddr.ip().to_string());

    if let Some(ip_rule) = ip_rule.clone() {
        if matches!(ip_rule.ingress, configdb::RuleGress::Deny) {
            println!("dropping connection with {connaddr_friendly}, blocked by rule");
            return;
        }
        
        if matches!(general_config.ingress, configdb::GenericRuleGress::Deny) && !matches!(ip_rule.ingress, configdb::RuleGress::Allow) {
            println!("dropping connection with {connaddr_friendly}, blocked by rule");
            return;
        }
    } else if matches!(general_config.ingress, configdb::GenericRuleGress::Deny) {
        println!("dropping connection with {connaddr_friendly}, blocked by rule");
        return;
    }

    println!("new connection {connaddr_friendly}");

    match edge_server::find_edge_server() {
        Some(edge_info) => {
            procedure(conn, connaddr_friendly.clone(), &edge_info, &ip_rule, &general_config).await;
            edge_server::decrement_conn_count(edge_info.destination);
            println!("the connection with {}, closed", connaddr_friendly.clone());
        },
        None => {
            eprintln!("failed to find an edge server, dropping the connection");
        }
    }
}
