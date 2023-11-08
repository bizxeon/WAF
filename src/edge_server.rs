use notify::Watcher;

use crate::configdb;

lazy_static::lazy_static! {
    #[allow(non_upper_case_globals)]
    static ref EDGE_SERVERS_LISTS: std::sync::Arc<std::sync::Mutex<Vec<(usize, configdb::Edge)>>> = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
}

pub fn decrement_conn_count(edge_server_ip: String) {
    match EDGE_SERVERS_LISTS.lock() {
        Ok(mut edge_server_list) => {
            for edge_server in edge_server_list.iter_mut() {
                if edge_server.1.destination == edge_server_ip {
                    edge_server.0 = edge_server.0 - 1;
                    break;
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lock EDGE_SERVERS_LISTS, error: {}; aborting", err.to_string());
            std::process::abort();
        }
    }
}

pub fn find_edge_server() -> Option<configdb::Edge> {
    let mut result: Option<configdb::Edge> = None;

    match EDGE_SERVERS_LISTS.lock() {
        Ok(mut edge_server_list) => {
            edge_server_list.sort_by(|a, b| { a.0.cmp(&b.0) });

            if let Some(_) = edge_server_list.get(0) {
                edge_server_list[0].0 = edge_server_list[0].0 + 1; // increment the conn count
                result = Some(edge_server_list[0].1.clone());
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lock EDGE_SERVERS_LISTS, error: {}; aborting", err.to_string());
            std::process::abort();
        }
    }
    
    result
}

fn load_edge_servers() {
    match EDGE_SERVERS_LISTS.lock() {
        Ok(mut edge_server_list) => {
            println!("loading edge servers");

            match std::fs::read_dir(configdb::EDGE_SERVER_DIRNAME) {
                Ok(dir) => {
                    for file in dir {
                        if let Ok(file) = file {
                            if let Some(filename) = file.file_name().to_str() {
                                let filename = format!("{}/{}", configdb::EDGE_SERVER_DIRNAME, filename);
        
                                match std::fs::read_to_string(&filename) {
                                    Ok(content) => {
                                        match serde_yaml::from_str::<configdb::Edge>(&content) {
                                            Ok(object) => {
                                                let mut in_list = false;
                                                for edge_server in edge_server_list.iter_mut() {
                                                    if edge_server.1.destination == object.destination && edge_server.1.destination_port == object.destination_port {
                                                        edge_server.1 = object.clone();
                                                        in_list = true;
                                                        break;
                                                    }
                                                }

                                                if !in_list {
                                                    edge_server_list.push((0, object));
                                                }
                                            },
                                            Err(err) => {
                                                eprintln!("failed to deserialize {}, error: {}", &filename, err.to_string());
                                            }
                                        }
                                    },
                                    Err(err) => {
                                        eprintln!("failed to access {}, error: {}", &filename, err.to_string());
                                    }
                                }
                            }
                        }
                    }
                },
                Err(err) => {
                    eprintln!("failed to enumerate the folder {}, error: {}; aborting", configdb::EDGE_SERVER_DIRNAME, err.to_string());
                    std::process::abort();
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lock EDGE_SERVERS_LISTS, error: {}; aborting", err.to_string());
            std::process::abort();
        }
    }
}

fn folder_watch() {
    let watcher = notify::recommended_watcher(|res: notify::Result<notify::Event>| {
        match res {
            Ok(_) => {
                load_edge_servers();
            },
            Err(err) => {
                eprintln!("failed to monitor the folder {} for update events, error: {}; aborting", configdb::EDGE_SERVER_DIRNAME, err.to_string());
                std::process::abort();
            }
        }
    });

    match watcher {
        Ok(mut watcher) => {
            if let Err(err) = watcher.watch(std::path::Path::new(configdb::EDGE_SERVER_DIRNAME), notify::RecursiveMode::Recursive) {
                eprintln!("failed to monitor the folder {} for update events, error: {}; aborting", configdb::EDGE_SERVER_DIRNAME, err.to_string());
                std::process::abort();
            }
        },
        Err(err) => {
            eprintln!("failed to monitor the folder {} for update events, error: {}; aborting", configdb::EDGE_SERVER_DIRNAME, err.to_string());
            std::process::abort();
        }
    }
}


pub fn initialize() {
    load_edge_servers();

    std::thread::spawn(|| {
        folder_watch();
    });
}

/*
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
*/