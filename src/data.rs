use futures::StreamExt;
use mongodb::bson::document::Document;
use mongodb::{Client, Cursor};

#[derive(Clone)]
pub struct DataManager {
    client: Client,
    db: String,
    collection: String,
}

impl DataManager {
    pub fn new(client: Client) -> DataManager {
        DataManager {
            client,
            db: "natssync".to_string(),
            collection: "locations".to_string(),
        }
    }

    pub async fn clients(&self) -> Vec<String> {
        log::debug!("Searching database {} collection {} for clients", self.db, self.collection);
        let pub_keys = self.client.database(&self.db).collection(&self.collection);
        let mut cursor: Cursor = pub_keys.find(None, None).await.unwrap();
        let mut clients = Vec::new();

        while let Some(result) = cursor.next().await {
            let doc: Document = result.unwrap();
            log::info!("Found doc: {:?}", doc);
            let location_id = doc.get_str("locationID").unwrap();
            if location_id == "cloud-master" {
                continue;
            }
            clients.push(location_id.to_string());
        }

        log::debug!("Found {} clients", clients.len());
        clients
    }
}
