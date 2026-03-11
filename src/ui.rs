use anyhow::{Context, Result, anyhow, bail};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use serde_json::json;

use crate::providers::linear;

pub(crate) struct LinearDestination {
    pub(crate) team_id: String,
    pub(crate) project_id: Option<String>,
}

pub(crate) fn prompt_drive_folder_id() -> Result<Option<String>> {
    let folder_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("🗂️ Drive folder ID (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()
        .context("Failed to read folder ID")?;

    let trimmed = folder_id.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

pub(crate) fn pick_linear_destination() -> Result<LinearDestination> {
    let teams_query = r#"
      query Teams {
        teams {
          nodes { id name }
        }
      }
    "#;
    let teams_data = linear::graphql(teams_query, json!({}))?;
    let teams = teams_data["teams"]["nodes"]
        .as_array()
        .ok_or_else(|| anyhow!("Invalid teams response"))?;

    if teams.is_empty() {
        bail!("No Linear team found");
    }

    let team_items: Vec<String> = teams
        .iter()
        .map(|team| team["name"].as_str().unwrap_or("unnamed").to_string())
        .collect();

    let team_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("🧠 Which Linear team should the doc be created in?")
        .items(&team_items)
        .default(0)
        .interact()
        .context("Failed to read team selection prompt")?;

    let team_id = teams[team_index]["id"]
        .as_str()
        .ok_or_else(|| anyhow!("Team is missing an id"))?
        .to_string();

    let projects_query = r#"
      query Projects {
        projects(filter: { state: { eq: "started" } }) {
          nodes { id name }
        }
      }
    "#;
    let projects_data = linear::graphql(projects_query, json!({}))?;
    let projects = projects_data["projects"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut project_options = vec!["(no project)".to_string()];
    project_options.extend(
        projects
            .iter()
            .map(|project| project["name"].as_str().unwrap_or("unnamed").to_string()),
    );

    let project_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("📌 Which Linear project? (optional)")
        .items(&project_options)
        .default(0)
        .interact()
        .context("Failed to read project selection prompt")?;

    let project_id = if project_index == 0 {
        None
    } else {
        projects
            .get(project_index - 1)
            .and_then(|project| project["id"].as_str())
            .map(|id| id.to_string())
    };

    Ok(LinearDestination {
        team_id,
        project_id,
    })
}
