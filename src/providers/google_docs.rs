use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::domain::document::GDoc;
use crate::domain::markdown::normalize_newlines;

pub(crate) fn doc_id_from_url(url: &str) -> Option<String> {
    let marker = "/document/d/";
    let start = url.find(marker)?;
    let remaining = &url[(start + marker.len())..];
    remaining.split('/').next().map(ToOwned::to_owned)
}

pub(crate) fn create_document(title: &str, folder_id: Option<&str>) -> Result<GDoc> {
    let token = google_token()?;
    let client = Client::new();
    let response: Value = client
        .post("https://docs.googleapis.com/v1/documents")
        .bearer_auth(&token)
        .json(&json!({ "title": title }))
        .send()
        .context("Failed to create Google Doc")?
        .error_for_status()
        .context("Google Docs API error (create)")?
        .json()
        .context("Invalid Google Docs API response")?;

    let id = response["documentId"]
        .as_str()
        .ok_or_else(|| anyhow!("documentId missing from Google response"))?
        .to_string();
    let url = format!("https://docs.google.com/document/d/{id}/edit");

    if let Some(folder_id) = folder_id {
        let drive_url = format!(
            "https://www.googleapis.com/drive/v3/files/{id}?addParents={folder_id}&fields=id,parents"
        );

        client
            .patch(drive_url)
            .bearer_auth(&token)
            .send()
            .context("Failed to move doc to Drive folder")?
            .error_for_status()
            .context("Google Drive API error (move)")?;
    }

    Ok(GDoc {
        url,
        title: title.to_string(),
        text: String::new(),
    })
}

pub(crate) fn write_document(doc_id: &str, text: &str) -> Result<()> {
    let token = google_token()?;
    let client = Client::new();

    let current_document: Value = client
        .get(format!("https://docs.googleapis.com/v1/documents/{doc_id}"))
        .bearer_auth(&token)
        .send()
        .context("Failed to read current doc")?
        .error_for_status()
        .context("Google Docs API error (get)")?
        .json()
        .context("Invalid get document response")?;

    let end_index = current_document["body"]["content"]
        .as_array()
        .and_then(|items| items.last())
        .and_then(|item| item["endIndex"].as_i64())
        .unwrap_or(2);

    let mut requests = Vec::new();
    if end_index > 2 {
        requests.push(json!({
            "deleteContentRange": {
                "range": { "startIndex": 1, "endIndex": end_index - 1 }
            }
        }));
    }
    requests.push(json!({
        "insertText": {
            "location": { "index": 1 },
            "text": text
        }
    }));

    client
        .post(format!(
            "https://docs.googleapis.com/v1/documents/{doc_id}:batchUpdate"
        ))
        .bearer_auth(&token)
        .json(&json!({ "requests": requests }))
        .send()
        .context("Failed to update Google Doc content")?
        .error_for_status()
        .context("Google Docs API error (batchUpdate)")?;

    Ok(())
}

pub(crate) fn get_document(doc_id: &str) -> Result<GDoc> {
    let token = google_token()?;
    let client = Client::new();
    let response: Value = client
        .get(format!("https://docs.googleapis.com/v1/documents/{doc_id}"))
        .bearer_auth(&token)
        .send()
        .context("Failed to fetch Google Doc")?
        .error_for_status()
        .context("Google Docs API error (get)")?
        .json()
        .context("Invalid Google Docs response")?;

    let title = response["title"].as_str().unwrap_or("document").to_string();
    let mut text = String::new();

    if let Some(items) = response["body"]["content"].as_array() {
        for item in items {
            if let Some(elements) = item["paragraph"]["elements"].as_array() {
                for element in elements {
                    if let Some(content) = element["textRun"]["content"].as_str() {
                        text.push_str(content);
                    }
                }
            }
        }
    }

    Ok(GDoc {
        url: format!("https://docs.google.com/document/d/{doc_id}/edit"),
        title,
        text: normalize_newlines(&text),
    })
}

fn google_token() -> Result<String> {
    std::env::var("GOOGLE_ACCESS_TOKEN")
        .map_err(|_| anyhow!("Set GOOGLE_ACCESS_TOKEN with a valid Google bearer token"))
}

#[cfg(test)]
mod tests {
    use super::doc_id_from_url;

    #[test]
    fn extracts_doc_id_from_google_url() {
        let id = doc_id_from_url("https://docs.google.com/document/d/abc123/edit");
        assert_eq!(id.as_deref(), Some("abc123"));
    }
}
