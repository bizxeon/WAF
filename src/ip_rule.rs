
use crate::configdb;

pub fn get_ip_rule(ip: String) -> Option<configdb::IpRule> {
    let rule_filename = format!("{}/{}.yaml", configdb::IP_RULES_DIRNAME, ip);

    match std::fs::read_to_string(&rule_filename) {
        Ok(content) => {
            match serde_yaml::from_str::<configdb::IpRule>(&content) {
                Ok(ip_rule) => {
                    return Some(ip_rule);
                },
                Err(err) => {
                    eprintln!("failed to deserialize '{}', error: {}", &rule_filename, err.to_string());    

                    return None;
                }
            }
        },
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!("failed to read from '{}', error: {}", &rule_filename, err.to_string());
            }


            return None;
        }
    }
}
