use crate::config::Config;
use crate::data::DataManager;
use crate::nats_connection::new_async;
use crate::test_manager::{
    batch_test_flood_clients, test_client_mps, test_client_response, StatsCollection, StressTest,
    TestResults,
};
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use async_nats as nats;
use futures::future;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct NewTest {
    clients: Vec<String>,
    test_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorMsg {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClientCollection {
    clients: Vec<String>,
    count: usize
}

async fn handle_client_test(
    nc: web::Data<nats::Connection>,
    client: web::Path<String>,
) -> impl Responder {
    let result = test_client_response(client.as_ref(), nc.get_ref()).await;
    let count = match result {
        Ok(_) => 1,
        Err(_) => 0,
    };
    let test_result = TestResults::new(client.to_string(), result.is_ok(), count);
    HttpResponse::Ok().json(test_result)
}

async fn handle_batch_test(
    nc: web::Data<nats::Connection>,
    new_test: web::Json<NewTest>,
) -> impl Responder {
    if new_test.clients.is_empty() || new_test.test_count < 1 {
        let error = ErrorMsg {
            message: "No clients provided to test".to_string(),
        };
        return HttpResponse::BadRequest().json(error);
    }

    let test = StressTest::new(new_test.clients.clone(), new_test.test_count);
    log::debug!("New test request: {:?}", new_test);

    let result = batch_test_flood_clients(test, nc.get_ref()).await;
    HttpResponse::Ok().json(result)
}

async fn handle_mps_test(
    nc: web::Data<nats::Connection>,
    client: web::Path<String>,
) -> impl Responder {
    let results = test_client_mps(&nc, &client).await;
    HttpResponse::Ok().json(results)
}

async fn handle_client_mps_test(
    nc: web::Data<nats::Connection>,
    config: web::Data<Config>,
) -> impl Responder {
    log::debug!("Handling client mps test");
    let data = DataManager::new(&config).await;
    let clients = data.clients().await;
    let mut test_futures = Vec::new();
    for client_id in clients.iter() {
        let f = test_client_mps(&nc, client_id);
        test_futures.push(f);
    }
    let results = future::join_all(test_futures).await;
    let collection = StatsCollection::new(results);
    HttpResponse::Ok().json(collection)
}

async fn handle_get_clients(config: web::Data<Config>) -> impl Responder {
    let data = DataManager::new(&config).await;
    let clients = data.clients().await;
    let collection = ClientCollection{count: clients.len(), clients};
    HttpResponse::Ok().json(collection)
}

async fn hello() -> impl Responder {
    "Hello, server!"
}

pub async fn run(config: Config) -> std::io::Result<()> {
    log::info!("Starting up webserver");

    let nc = new_async(&config.nats_url).await;
    let port = config.web_port;

    HttpServer::new(move || {
        App::new()
            .data(nc.clone())
            .data(config.clone())
            .wrap(middleware::Logger::default())
            .route("/", web::get().to(hello))
            .route("/clients", web::get().to(handle_get_clients))
            .route("/tests", web::post().to(handle_batch_test))
            .route("/tests/mps", web::get().to(handle_client_mps_test))
            .route("/tests/{client}", web::get().to(handle_client_test))
            .route("/tests/{client}/mps", web::get().to(handle_mps_test))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
