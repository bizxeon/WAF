
#[derive(Clone, Default)]
pub struct Http {
    pub location: String,
    pub method: String,
    pub properties: std::collections::HashMap<String, String>
}

pub fn parse(block: Vec<u8>) -> Result<Http, std::io::Error> {
    match String::from_utf8(block) {
        Ok(block) => {
            let mut result = Http::default();

            for (idx, line) in block.trim().lines().enumerate() {
                let line = line.trim();

                match idx {
                    0 => {
                        let storage: Vec<&str> = line.splitn(3, ' ').collect();

                        match storage.get(0) {
                            Some(method) => {
                                match storage.get(1) {
                                    Some(location) => {
                                        match storage.get(2) {
                                            Some(protocol) => {
                                                if *protocol != "HTTP/1.1" {
                                                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "unsupported HTTP version"));    
                                                }

                                                result.method = method.to_string();
                                                result.location = location.to_string();
                                            },
                                            None => {                            
                                                return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("faulty line is '{}'", line)));
                                            }
                                        }
                                    },
                                    None => {                            
                                        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("faulty line is '{}'", line)));
                                    }
                                }
                            },
                            None => {                            
                                return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("faulty line is '{}'", line)));
                            }
                        }
                    },
                    _ => {
                        let storage: Vec<&str> = line.splitn(2, ':').collect();

                        if storage.len() == 2 {
                            result.properties.insert(storage[0].trim().to_string(), storage[1].trim().to_string());
                        } else {
                            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("faulty line is '{}'", line)));
                        }
                    }
                }
            }

            return Ok(result);
        },
        Err(err) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
        }
    }
}
