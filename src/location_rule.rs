use notify::Watcher;

use crate::configdb;

lazy_static::lazy_static! {
    #[allow(non_upper_case_globals)]
    static ref LOCATION_LISTS: std::sync::Arc<std::sync::Mutex<Vec<configdb::LocationRule>>> = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
}

pub fn get_location_rule<Method: AsRef<str>, Location: AsRef<str>>(method: Method, location: Location) -> Option<configdb::LocationRule> {
    let mut result: Option<configdb::LocationRule> = None;

    match LOCATION_LISTS.lock() {
        Ok(location_list) => {
            for rule in location_list.iter() {
                if rule.method.as_str() == method.as_ref() && rule.location.as_str() == location.as_ref() {
                    result = Some((*rule).clone());
                    break;
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lock LOCATION_LISTS, error: {}; aborting", err.to_string());
            std::process::abort();
        }
    }
    
    result
}

fn load_rules() {
    match LOCATION_LISTS.lock() {
        Ok(mut location_list) => {
            println!("loading location rules");

            match std::fs::read_dir(configdb::LOCATION_RULES_DIRNAME) {
                Ok(dir) => {
                    for file in dir {
                        if let Ok(file) = file {
                            if let Some(filename) = file.file_name().to_str() {
                                let filename = format!("{}/{}", configdb::LOCATION_RULES_DIRNAME, filename);
        
                                match std::fs::read_to_string(&filename) {
                                    Ok(content) => {
                                        match serde_yaml::from_str::<configdb::LocationRule>(&content) {
                                            Ok(object) => {
                                                location_list.push(object);
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
                    eprintln!("failed to enumerate the folder {}, error: {}; aborting", configdb::LOCATION_RULES_DIRNAME, err.to_string());
                    std::process::abort();
                }
            }
        },
        Err(err) => {
            eprintln!("internal error, failed to lock LOCATION_LISTS, error: {}; aborting", err.to_string());
            std::process::abort();
        }
    }
}

fn folder_watch() {
    let watcher = notify::recommended_watcher(|res: notify::Result<notify::Event>| {
        match res {
            Ok(_) => {
                load_rules();
            },
            Err(err) => {
                eprintln!("failed to monitor the folder {} for update events, error: {}; aborting", configdb::LOCATION_RULES_DIRNAME, err.to_string());
                std::process::abort();
            }
        }
    });

    match watcher {
        Ok(mut watcher) => {
            if let Err(err) = watcher.watch(std::path::Path::new(configdb::LOCATION_RULES_DIRNAME), notify::RecursiveMode::Recursive) {
                eprintln!("failed to monitor the folder {} for update events, error: {}; aborting", configdb::LOCATION_RULES_DIRNAME, err.to_string());
                std::process::abort();
            }
        },
        Err(err) => {
            eprintln!("failed to monitor the folder {} for update events, error: {}; aborting", configdb::LOCATION_RULES_DIRNAME, err.to_string());
            std::process::abort();
        }
    }
}

pub fn initialize() {
    load_rules();

    std::thread::spawn(|| {
        folder_watch();
    });
}
