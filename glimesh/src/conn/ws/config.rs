use std::time::Duration;

#[derive(Debug)]
pub(super) struct Config {
    pub api_url: String,
    pub version: String,
    pub outgoing_capacity: usize,
    pub incoming_capacity: usize,
    pub ping_interval: Duration,
    pub request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_url: String::from("wss://glimesh.tv/api/socket/websocket"),
            version: String::from("2.0.0"),
            outgoing_capacity: 100,
            incoming_capacity: 10_000,
            ping_interval: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
        }
    }
}
