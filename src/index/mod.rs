use std::{fs, sync::Mutex};

use meilisearch_sdk::Client;
use postgres_queue::{TaskData, TaskError};
use serde_json::Value;
use tokio::task::spawn_blocking;
use tracing::info;

use crate::graph::Email;

pub async fn full_index_handler_sync(task_id: i32, task_data: TaskData) -> Result<(), TaskError> {
    let fut = Mutex::new(Box::pin(full_index_handler(task_id, task_data)));
    spawn_blocking(move || {
        let mut guard = fut.lock().unwrap();
        futures::executor::block_on(&mut *guard)
    })
    .await
    .map_err(|e| TaskError::Custom(e.to_string()))?
}

pub async fn full_index_handler(_task_id: i32, task_data: TaskData) -> Result<(), TaskError> {
    // let access_token = task_data.get("access_token").unwrap().as_str().unwrap();
    info!("Full index handler called: {task_data:#?}");
    let user_id = task_data.get("user_id").unwrap().as_str().unwrap();

    let client = Client::new("http://localhost:7700", "masterKey");

    let json = fs::read_to_string("src/fixtures/broken-sender.json").unwrap();
    let json: Value = serde_json::from_str(&json).unwrap();
    let json = json["value"].as_array().unwrap();
    let json = json.get(0).unwrap();
    let email: Email = serde_json::from_value(json.clone()).unwrap();
    let emails = vec![email];

    // Fetch emails from Microsoft Graph API, using access_token and user_id
    // let emails = fetch_emails_from_graph_api(access_token, user_id)
    //     .await
    //     .unwrap();

    // Add emails to Meilisearch
    let result = client
        .index(user_id)
        .add_documents(&emails, None)
        .await
        .unwrap();
    info!("Meilisearch result: {:#?}", result);

    Ok(())
}
