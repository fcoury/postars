use std::sync::Mutex;

use base64::{encode_config, URL_SAFE_NO_PAD};
use meilisearch_sdk::Client;
use postgres_queue::{TaskData, TaskError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;
use tracing::info;

use crate::{
    database::{Database, User},
    graph::GraphClient,
};

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
    let user_email = task_data.get("user_email").unwrap().as_str().unwrap();
    let has_pagination = task_data.get("num_pages").is_some();
    let start_page = match task_data.get("start_page") {
        Some(page) => page.as_i64().unwrap(),
        None => 0,
    };
    let num_pages = match task_data.get("num_pages") {
        Some(page) => page.as_i64().unwrap(),
        None => 0,
    };

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let database = Database::new(database_url).await.unwrap();
    let client = database.get().await.unwrap();
    let user = User::find(&client, user_email).await.unwrap().unwrap();

    let Some(token) = user.access_token else {
        return Err(TaskError::Custom("No access token".to_string()));
    };

    let client = Client::new("http://localhost:7700", "masterKey");
    let graph = GraphClient::new(token);

    let (emails, has_more) = if has_pagination {
        graph
            .get_user_emails_paginated(start_page as usize, num_pages as usize)
            .await
            .unwrap()
    } else {
        (graph.get_user_emails().await.unwrap(), false)
    };

    let documents = emails
        .into_iter()
        .map(|email| {
            let mut json = serde_json::to_value(email).unwrap();
            let id = json["id"].as_str().unwrap();
            let unique_id = generate_deterministic_key(id);
            json.as_object_mut()
                .unwrap()
                .insert("uniqueId".to_string(), Value::String(unique_id));
            json
        })
        .collect::<Vec<Value>>();

    info!(
        "Indexing {} emails. Has more? {}",
        documents.len(),
        has_more
    );

    // Add emails to Meilisearch
    let result = client
        .index(format!("emails_{}", user.id.unwrap()))
        .add_documents(&documents, Some("uniqueId"))
        .await
        .unwrap();
    info!("Meilisearch result: {:#?}", result);

    Ok(())
}
