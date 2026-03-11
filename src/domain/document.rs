use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Frontmatter {
    #[serde(rename = "gdocUrl", skip_serializing_if = "Option::is_none")]
    pub(crate) gdoc_url: Option<String>,
    #[serde(rename = "linearDocUrl", skip_serializing_if = "Option::is_none")]
    pub(crate) linear_doc_url: Option<String>,
    #[serde(rename = "linearDocId", skip_serializing_if = "Option::is_none")]
    pub(crate) linear_doc_id: Option<String>,
    #[serde(rename = "gitUrl", skip_serializing_if = "Option::is_none")]
    pub(crate) git_url: Option<String>,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct MdDoc {
    pub(crate) path: PathBuf,
    pub(crate) frontmatter: Frontmatter,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LinearDoc {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct GDoc {
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) text: String,
}
