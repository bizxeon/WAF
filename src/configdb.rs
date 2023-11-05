use serde::{Deserialize, Serialize};

pub const GENERAL_CONFIG_FILENAME: &str = "appdata/general.yaml";
pub const EDGE_SERVER_DIRNAME: &str = "appdata/edges/";

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ConfigEdge {
    pub destination: String,
    pub destination_port: u16,
    pub resolve_name: String,
    pub maximum_number_of_conn: usize,
    pub conn_count: usize,
    pub requests_per_second: usize,
    pub https: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ConfigGeneral {
    pub listen_address: String,
    pub listen_port: u16,
    pub maximum_connections: usize,
    pub https: bool,
    pub ssl_certificate: String,
    pub ssl_certificate_key: String,
}
