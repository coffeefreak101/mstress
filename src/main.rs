mod config;
mod data;
mod nats_connection;
mod server;
mod test_manager;

use crate::config::Config;
use env_logger as logger;
use log;

fn init_logger() {
    logger::Builder::new()
        .target(logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .init();
}

#[actix_web::main]
async fn main() {
    init_logger();

    let config = Config::new();
    let result = server::run(config).await;

    match result {
        Ok(_) => {
            log::info!("Service stopped");
        }
        Err(error) => {
            log::error!("Service error: {}", error);
        }
    };
}
