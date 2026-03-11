use anyhow::{Context, Result, anyhow, bail};
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::domain::document::{Frontmatter, LinearDoc};

pub(crate) fn doc_id_from_frontmatter(frontmatter: &Frontmatter) -> Option<String> {
    if let Some(id) = &frontmatter.linear_doc_id {
        return Some(id.clone());
    }

    let url = frontmatter.linear_doc_url.as_deref()?;
    let parts: Vec<&str> = url.split('/').collect();
    let position = parts.iter().position(|part| *part == "document")?;
    parts.get(position + 1).map(|id| (*id).to_string())
}

pub(crate) fn graphql(query: &str, variables: Value) -> Result<Value> {
    let key = linear_api_key()?;
    let client = Client::new();
    let response: Value = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", key)
        .json(&json!({ "query": query, "variables": variables }))
        .send()
        .context("Network failure calling Linear API")?
        .error_for_status()
        .context("HTTP error from Linear API")?
        .json()
        .context("Invalid Linear API response")?;

    if let Some(errors) = response.get("errors") {
        bail!("Linear GraphQL error: {}", errors);
    }

    Ok(response["data"].clone())
}

pub(crate) fn get_document(id: &str) -> Result<LinearDoc> {
    let query = r#"
      query GetDocument($id: String!) {
        document(id: $id) {
          id
          url
          title
          content
        }
      }
    "#;
    let data = graphql(query, json!({ "id": id }))?;
    let document = &data["document"];

    Ok(LinearDoc {
        id: document["id"].as_str().unwrap_or(id).to_string(),
        url: document["url"].as_str().unwrap_or_default().to_string(),
        title: document["title"].as_str().unwrap_or("document").to_string(),
        content: document["content"].as_str().unwrap_or("").to_string(),
    })
}

pub(crate) fn update_document(id: &str, content: &str) -> Result<()> {
    let query = r#"
      mutation UpdateDocument($id: String!, $content: String!) {
        updateDocument(id: $id, input: { content: $content }) {
          success
        }
      }
    "#;

    graphql(query, json!({ "id": id, "content": content }))?;
    Ok(())
}

pub(crate) fn create_document(
    title: &str,
    content: &str,
    team_id: &str,
    project_id: Option<&str>,
) -> Result<LinearDoc> {
    let query = r#"
      mutation CreateDocument($title: String!, $content: String!, $teamId: String!, $projectId: String) {
        createDocument(input: { title: $title, content: $content, teamId: $teamId, projectId: $projectId }) {
          success
          document {
            id
            url
            title
            content
          }
        }
      }
    "#;
    let data = graphql(
        query,
        json!({
            "title": title,
            "content": content,
            "teamId": team_id,
            "projectId": project_id
        }),
    )?;
    let document = &data["createDocument"]["document"];

    Ok(LinearDoc {
        id: document["id"].as_str().unwrap_or_default().to_string(),
        url: document["url"].as_str().unwrap_or_default().to_string(),
        title: document["title"].as_str().unwrap_or(title).to_string(),
        content: document["content"].as_str().unwrap_or(content).to_string(),
    })
}

fn linear_api_key() -> Result<String> {
    std::env::var("LINEAR_API_KEY")
        .map_err(|_| anyhow!("Set LINEAR_API_KEY with a valid Linear token"))
}

#[cfg(test)]
mod tests {
    use super::doc_id_from_frontmatter;
    use crate::domain::document::Frontmatter;

    #[test]
    fn prioritizes_explicit_linear_id() {
        let frontmatter = Frontmatter {
            linear_doc_id: Some("abc".to_string()),
            ..Frontmatter::default()
        };

        assert_eq!(
            doc_id_from_frontmatter(&frontmatter).as_deref(),
            Some("abc")
        );
    }

    #[test]
    fn extracts_linear_id_from_url() {
        let frontmatter = Frontmatter {
            linear_doc_url: Some(
                "https://linear.app/workspace/document/abc123/title-slug".to_string(),
            ),
            ..Frontmatter::default()
        };

        assert_eq!(
            doc_id_from_frontmatter(&frontmatter).as_deref(),
            Some("abc123")
        );
    }
}
