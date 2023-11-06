use serde::{Deserialize, Serialize};

pub const GENERAL_CONFIG_FILENAME: &str = "appdata/general.yaml";
pub const EDGE_SERVER_DIRNAME: &str = "appdata/edges/";
pub const IP_RULES_DIRNAME: &str = "appdata/ip-rules/";

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub enum RuleGress {
    #[default]
    GenericRule,
    Allow,
    Deny,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub enum GenericRuleGress {
    #[default]
    Allow,
    Deny,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct IpRule {
    pub ip: String,
    pub ingress: RuleGress,
    pub bypass_protection: bool,
    pub limit_rate: usize,
    pub blacklisted_locations: Vec<String>,
    pub whitelist_location: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Edge {
    pub destination: String,
    pub destination_port: u16,
    pub resolve_name: String,
    pub maximum_number_of_conn: usize,
    pub conn_count: usize,
    pub requests_per_second: usize,
    pub https: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct General {
    pub listen_address: String,
    pub listen_port: u16,
    pub maximum_connections: usize,
    pub https: bool,
    pub ssl_certificate: String,
    pub ssl_certificate_key: String,
    pub ingress: GenericRuleGress,
}
