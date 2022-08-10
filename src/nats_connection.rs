use async_nats;

pub async fn new_async(nats_url: &str) -> async_nats::Connection {
    loop {
        let nc_result = async_nats::Options::new()
            .with_name("mstress-server")
            .close_callback(|| log::warn!("NATS connection closed"))
            .disconnect_callback(|| log::warn!("NATS connection disconnected"))
            .reconnect_callback(|| log::info!("NATS connection reconnected"))
            .connect(&nats_url)
            .await;
        match nc_result {
            Ok(async_nc) => {
                log::info!("Connected to NATS at {}", nats_url);
                return async_nc;
            }
            Err(error) => {
                log::error!("Error connecting to NATS: {}", error);
            }
        };
    }
}
