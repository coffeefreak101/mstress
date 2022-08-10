use async_nats as nats;
use async_std::future::timeout;
use futures::executor::block_on;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, SystemTime};
use std::{io, thread};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StressTest {
    pub id: Uuid,
    pub clients: Vec<String>,
    pub count: usize,
}

impl StressTest {
    pub fn new(clients: Vec<String>, count: usize) -> StressTest {
        let id = Uuid::new_v4();
        StressTest { id, clients, count }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TestMessage {
    pub id: usize,
    pub time: SystemTime,
    pub client: String,
}

impl TestMessage {
    pub fn new(id: usize, client: &String) -> TestMessage {
        TestMessage {
            id,
            client: client.clone(),
            time: SystemTime::now(),
        }
    }

    pub fn from_utf8(data: &[u8]) -> io::Result<TestMessage> {
        let msg_str = match std::str::from_utf8(data) {
            Ok(msg) => msg,
            Err(error) => {
                log::error!("Failure to parse message: {}", error);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    error.to_string(),
                ));
            }
        };
        let test_msg: TestMessage = serde_json::from_str(msg_str)?;
        Ok(test_msg)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestResults {
    pub client: String,
    pub success: bool,
    pub response_count: usize,
}

impl TestResults {
    pub fn new(client: String, success: bool, response_count: usize) -> TestResults {
        TestResults {
            client,
            success,
            response_count,
        }
    }
}

fn make_request_subject(client: &String) -> String {
    format!("natssyncmsg.{}.echo", client)
}

fn make_response_subject(client_id: &String, subject: &String) -> String {
    format!("natssyncmsg.cloud-master.{}.{}", client_id, subject)
}

fn make_echolet_response_subject(subject: &String) -> String {
    format!("{}.echolet", subject)
}

pub async fn test_client_response(client: &String, nc: &nats::Connection) -> io::Result<()> {
    log::info!("Testing response from client {}", client);
    let response_subject = make_response_subject(client, &nc.new_inbox());
    let response_subscribe = make_echolet_response_subject(&response_subject);
    let sub = nc.subscribe(&response_subscribe).await?;
    let msg = TestMessage::new(1, client);
    let msg_str = msg.to_string();

    nc.publish_request(
        &make_request_subject(client),
        &response_subject,
        msg_str.as_bytes(),
    ).await?;
    log::info!("Published request to {}: {}", client, msg_str);

    let result_future = sub.next();
    let result = timeout(Duration::from_secs(5), result_future).await;
    let _ = sub.unsubscribe().await;

    match result {
        Ok(Some(resp)) => {
            log::info!("Response success for client {}: {:?}", client, resp);
            Ok(())
        }
        Ok(None) => {
            let msg = format!("No response from client {}", client);
            log::error!("{}", msg);
            Err(io::Error::new(io::ErrorKind::NotConnected, msg))
        }
        Err(error) => {
            log::error!("Timeout waiting for client {}", client);
            Err(io::Error::new(io::ErrorKind::TimedOut, error.to_string()))
        }
    }
}

pub async fn batch_test_flood_clients(test: StressTest, nc: &nats::Connection) -> Vec<TestResults> {
    let response_subject = make_response_subject(&test.clients[0], &nc.new_inbox());
    let echolet_response_subject = make_echolet_response_subject(&response_subject);

    let sub = nc.subscribe(&echolet_response_subject).await.unwrap();

    let messages: Vec<(String, Vec<u8>)> = test
        .clients
        .iter()
        .map(|c| {
            (
                make_request_subject(c),
                TestMessage::new(1, c).to_string().into_bytes(),
            )
        })
        .collect();
    let mut all_messages = Vec::new();

    for _ in 0..test.count {
        all_messages.extend(messages.clone());
    }

    let mut all_futures = Vec::new();
    for (subject, msg) in all_messages.iter() {
        all_futures.push(nc.publish_request(subject, &response_subject, msg));
    }
    log::info!("Sending {} test messages", all_futures.len());

    let (tx, rx) = channel();
    thread::spawn(move || {
        block_on(start_collect_thread(
            test.clients.clone(),
            test.count,
            sub,
            tx,
        ))
    });

    join_all(all_futures).await;
    log::info!("Finished sending test messages");

    let responses = rx.recv().unwrap();

    log::debug!("Test responses: {:?}", responses);
    responses
}

async fn start_collect_thread(
    clients: Vec<String>,
    test_count: usize,
    sub: nats::Subscription,
    tx: Sender<Vec<TestResults>>,
) {
    let mut results: HashMap<String, TestResults> = clients
        .iter()
        .map(|c| (c.clone(), TestResults::new(c.clone(), false, 0)))
        .collect();
    let _ = timeout(
        Duration::from_secs(5),
        collect_responses(&sub, &mut results, test_count),
    )
    .await;
    tx.send(results.values().cloned().collect()).unwrap();
    let _ = sub.unsubscribe().await;
}

async fn collect_responses(
    sub: &nats::Subscription,
    results: &mut HashMap<String, TestResults>,
    test_count: usize,
) {
    let expected_total = test_count * results.len();
    let mut resp_count_total = 0;
    loop {
        match sub.next().await {
            Some(msg) => {
                let msg = TestMessage::from_utf8(&msg.data).unwrap();
                results.get_mut(&msg.client).unwrap().response_count += 1;
                if results[&msg.client].response_count >= test_count {
                    results.get_mut(&msg.client).unwrap().success = true;
                }
                resp_count_total += 1;
                if resp_count_total >= expected_total && results.iter().all(|r| r.1.success) {
                    log::debug!("Breaking out of subscription loop");
                    break;
                }
            }
            None => {
                log::error!("Subscription connection closed");
                break;
            }
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestResultStats {
    client: String,
    count: usize,
    mps: f64,
}

impl TestResultStats {
    pub fn new(client: String) -> TestResultStats {
        TestResultStats {
            client,
            count: 0,
            mps: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatsCollection {
    results: Vec<TestResultStats>,
    min: f64,
    max: f64,
    average: f64,
}

impl StatsCollection {
    pub fn new(mut results: Vec<TestResultStats>) -> StatsCollection {
        let mut min = 0.0;
        let mut max = 0.0;
        let mut average = 0.0;
        let count = results.len();

        if count != 0 {
            results.sort_by(|a, b| a.mps.partial_cmp(&b.mps).unwrap());
            min = results.first().unwrap().mps;
            max = results.last().unwrap().mps;
            let total: f64 = results.iter().map(|t| t.mps).sum();
            average = total / count as f64;
        }

        StatsCollection {
            results,
            min,
            max,
            average,
        }
    }
}

pub async fn test_client_mps(nc: &nats::Connection, client_id: &String) -> TestResultStats {
    let timeout_secs = 10.0;
    let resp_subject = make_response_subject(&client_id, &nc.new_inbox());
    let echolet_response_subject = make_echolet_response_subject(&resp_subject);
    let mut test_results = TestResultStats::new(client_id.clone());
    let sub = nc
        .subscribe(&echolet_response_subject)
        .await
        .unwrap();

    let _ = timeout(
        Duration::from_secs(timeout_secs as u64),
        continuous_send_message(nc, &sub, &resp_subject, &mut test_results),
    )
    .await;
    test_results.mps = (test_results.count as f64) / timeout_secs;
    let _ = sub.unsubscribe().await;

    return test_results;
}

async fn continuous_send_message(
    nc: &nats::Connection,
    sub: &nats::Subscription,
    nb_subject: &String,
    test_results: &mut TestResultStats,
) {
    let mut msg_id = 0;
    let sb_subject = make_request_subject(&test_results.client);

    loop {
        let success = test_send_receive_with_id(nc, sub, &sb_subject, nb_subject, msg_id).await;
        if success {
            test_results.count += 1;
            msg_id += 1;
        }
    }
}

async fn test_send_receive_with_id(
    nc: &nats::Connection,
    sub: &nats::Subscription,
    subject: &String,
    reply: &String,
    msg_id: usize,
) -> bool {
    let msg = msg_id.to_string();
    log::info!("Sending msg with ID '{}' to subject {}", msg, subject);

    if let Err(err) = nc.publish_request(subject, reply, msg.as_bytes()).await {
        log::error!("Error publishing message {}", err);
        return false;
    }

    return match sub.next().await {
        Some(msg) => {
            let msg_data = String::from_utf8(msg.data).unwrap_or("Invalid data".to_string());
            log::info!("Received response: {:?}", msg_data);
            true
        }
        None => false,
    };
}
