use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::edge_server;

async fn procedure(mut conn: tokio::net::TcpStream, connaddr: String, edgeconnaddr: String) {
    let mut edge_conn = match tokio::net::TcpStream::connect(&edgeconnaddr).await {
        Ok(conn) => {
            conn
        },
        Err(err) => {
            eprintln!("failed to connect to {}, error: {}", edgeconnaddr, err.to_string());
            return;
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
                                eprintln!("failed to move data from client {} to edge server {}, error: {}; closing the connection", &connaddr, &edgeconnaddr, err.to_string());
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
                        eprintln!("edge {} closed the connection", &edgeconnaddr);
                        return;
                    },
                    Ok(len) => {
                        match conn.write(&edge_mtu_block[..len]).await {
                            Ok(_) => { },
                            Err(err) => {
                                eprintln!("failed to move data from edge {} to client {}, error: {}; closing the connection", &edgeconnaddr, &connaddr, err.to_string());
                                return;
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("failed to read from {}, error: {}; closing the connection", &edgeconnaddr, err.to_string());
                        return;
                    }
                }
            }
        }
    }
}

pub async fn handler(conn: tokio::net::TcpStream, connaddr: std::net::SocketAddr) {
    let _ = conn;
    let connaddr_friendly = connaddr.to_string();

    println!("new connection {connaddr_friendly}");

    match edge_server::find_edge_server() {
        Ok(edge_info) => {
            let edge_server_ip = edge_info.0;
            let edge_server_port = edge_info.1;

            procedure(conn, connaddr_friendly.clone(), format!("{}:{}", &edge_server_ip, edge_server_port)).await;
            edge_server::decrement_conn_count(edge_server_ip);
            println!("the connection with {}, closed", connaddr_friendly.clone());
        },
        Err(err) => {
            eprintln!("failed to find an edge server, error: {}, dropping the connection", err.to_string());
        }
    }
}
