use std::env;

pub struct Config {
    pub nats_url: String,
    pub mongo_url: String,
    pub web_port: usize,
}

impl Config {
    pub fn new() -> Config {
        Config {
            nats_url: env::var("NATS_URL").unwrap_or("nats://nats:4222".to_string()),
            mongo_url: env::var("MONGO_URL").unwrap_or("mongodb://mongo".to_string()),
            web_port: env::var("WEB_PORT")
                .unwrap_or("8080".to_string())
                .parse()
                .unwrap(),
        }
    }
}
