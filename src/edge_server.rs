use crate::configdb;

static EDGE_SERVER_LOCK: std::sync::Mutex<usize> = std::sync::Mutex::new(0);

pub fn decrement_conn_count(edge_server_ip: String) {
    let edge_filename = format!("{}.yaml", edge_server_ip);

    match EDGE_SERVER_LOCK.lock() {
        Ok(_) => {
            let edge_filepath = format!("{}/{}", configdb::EDGE_SERVER_DIRNAME, edge_filename);

            let mut edge_server = match std::fs::read_to_string(&edge_filepath) {
                Ok(edge_file_content) => {
                    match serde_yaml::from_str::<configdb::Edge>(&edge_file_content) {
                        Ok(edge_server) => {
                            edge_server
                        },
                        Err(err) => {
                            eprintln!("corrupted or invalid file {} from folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                            return;
                        }
                    }
                },
                Err(err) => {
                    eprintln!("failed to read file {} from folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                    return;
                }
            };

            if edge_server.conn_count > 0 {
                edge_server.conn_count = edge_server.conn_count - 1;

                match std::fs::write(&edge_filepath, serde_yaml::to_string(&edge_server).unwrap()) {
                    Ok(_) => { },
                    Err(err) => {
                        eprintln!("failed to update the file {}, error: {}", edge_filename, err.to_string());
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lcok EDGE_SERVER_LOCK, error: {}", err.to_string());
        }
    }
}

pub fn find_edge_server() -> Result<configdb::Edge, std::io::Error> {
    let mut edge_servers_list: Vec<(String, configdb::Edge)> = Vec::new();

    match EDGE_SERVER_LOCK.lock() {
        Ok(_) => {
            match std::fs::read_dir(configdb::EDGE_SERVER_DIRNAME) {
                Ok(directory) => {
                    for edge_file in directory {
                        match edge_file {
                            Ok(edge_file) => {
                                match edge_file.file_name().to_str() {
                                    Some(edge_filename) => {
                                        let edge_filepath = format!("{}/{}", configdb::EDGE_SERVER_DIRNAME, edge_filename);
        
                                        match std::fs::read_to_string(&edge_filepath) {
                                            Ok(edge_file_content) => {
                                                match serde_yaml::from_str::<configdb::Edge>(&edge_file_content) {
                                                    Ok(edge_server) => {
                                                        edge_servers_list.push((edge_filepath, edge_server));
                                                    },
                                                    Err(err) => {
                                                        eprintln!("corrupted or invalid file {} from folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                                                    }
                                                }
                                            },
                                            Err(err) => {
                                                eprintln!("failed to read file {} from folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                                            }
                                        }
                                    },
                                    None => {
                                        eprintln!("the folder {} contains empty filename", configdb::EDGE_SERVER_DIRNAME);
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("failed to enumerate a file from {}, error: {}", configdb::EDGE_SERVER_DIRNAME, err.to_string());
                            }
                        }
                    }
                },
                Err(err) => {
                    return Err(err);
                }
            }
        
            edge_servers_list.sort_by(|a, b| { a.1.conn_count.cmp(&b.1.conn_count) });
        
            match edge_servers_list.get(0) {
                Some(edge_server) => {
                    let filename = edge_server.0.clone();
                    let mut config_edge = edge_server.1.clone();
                    
                    config_edge.conn_count = config_edge.conn_count + 1;
        
                    match std::fs::write(&filename, serde_yaml::to_string(&config_edge).unwrap()) {
                        Ok(_) => {
                            return Ok(config_edge);
                        },
                        Err(err) => {
                            eprintln!("failed to update the file {}, error: {}", filename, err.to_string());
                            return Err(err);
                        }
                    }
                },
                None => {
                    eprintln!("no edge servers exists");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "no edge servers are present"));
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lcok EDGE_SERVER_LOCK, error: {}", err.to_string());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
        }
    }
}

pub fn initialize() -> Result<(), std::io::Error> {
    match EDGE_SERVER_LOCK.lock() {
        Ok(_) => {
            match std::fs::read_dir(configdb::EDGE_SERVER_DIRNAME) {
                Ok(directory) => {
                    for edge_file in directory {
                        match edge_file {
                            Ok(edge_file) => {
                                match edge_file.file_name().to_str() {
                                    Some(edge_filename) => {
                                        let edge_filepath = format!("{}/{}", configdb::EDGE_SERVER_DIRNAME, edge_filename);

                                        match std::fs::read_to_string(&edge_filepath) {
                                            Ok(edge_file_content) => {
                                                match serde_yaml::from_str::<configdb::Edge>(&edge_file_content) {
                                                    Ok(mut edge_server) => {
                                                        edge_server.conn_count = 0;

                                                        match std::fs::write(edge_filepath, serde_yaml::to_string(&edge_server).unwrap()) {
                                                            Ok(_) => {
                                                                return Ok(());
                                                            },
                                                            Err(err) => {
                                                                eprintln!("failed to reset the edge server's connection count for {} in folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                                                                return Err(err);
                                                            }
                                                        }
                                                    },
                                                    Err(err) => {
                                                        eprintln!("corrupted or invalid file {} from folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                                                    }
                                                }
                                            },
                                            Err(err) => {
                                                eprintln!("failed to read file {} from folder {}, error: {}", edge_filename, configdb::EDGE_SERVER_DIRNAME, err.to_string());
                                            }
                                        }
                                    },
                                    None => {
                                        eprintln!("the folder {} contains empty filename", configdb::EDGE_SERVER_DIRNAME);
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("failed to enumerate a file from {}, error: {}", configdb::EDGE_SERVER_DIRNAME, err.to_string());
                            }
                        }
                    }
                },
                Err(err) => {
                    return Err(err);
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lcok EDGE_SERVER_LOCK, error: {}", err.to_string());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
        }
    }

    Ok(())
}
