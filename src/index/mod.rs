use std::{fs, sync::Mutex};

use base64::{encode_config, URL_SAFE_NO_PAD};
use meilisearch_sdk::Client;
use postgres_queue::{TaskData, TaskError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;
use tracing::info;

pub async fn full_index_handler_sync(task_id: i32, task_data: TaskData) -> Result<(), TaskError> {
    let fut = Mutex::new(Box::pin(full_index_handler(task_id, task_data)));
    spawn_blocking(move || {
        let mut guard = fut.lock().unwrap();
        futures::executor::block_on(&mut *guard)
    })
    .await
    .map_err(|e| TaskError::Custom(e.to_string()))?
}

fn generate_deterministic_key(id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id);
    let hash = hasher.finalize();
    encode_config(hash, URL_SAFE_NO_PAD)
}

pub async fn full_index_handler(_task_id: i32, task_data: TaskData) -> Result<(), TaskError> {
    info!("Full index handler called: {task_data:#?}");
    let user_id = task_data.get("user_id").unwrap().as_str().unwrap();
    let file = task_data.get("file").unwrap().as_str().unwrap();

    let client = Client::new("http://localhost:7700", "masterKey");

    let json = fs::read_to_string(file).unwrap();
    let mut json: Value = serde_json::from_str(&json).unwrap();
    let json_array = json["value"].as_array_mut().unwrap();

    for entry in json_array.iter_mut() {
        let id = entry["id"].as_str().unwrap();
        let unique_id = generate_deterministic_key(id);
        entry
            .as_object_mut()
            .unwrap()
            .insert("uniqueId".to_string(), Value::String(unique_id));
    }

    info!("Indexing {} emails", json_array.len());

    // Add emails to Meilisearch
    let result = client
        .index(user_id)
        .add_documents(json_array, Some("uniqueId"))
        .await
        .unwrap();
    info!("Meilisearch result: {:#?}", result);

    Ok(())
}
