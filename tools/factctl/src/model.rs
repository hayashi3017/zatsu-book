use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Draft,
    Published,
    Duplicate,
    Superseded,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Official,
    Paper,
    Primary,
    Secondary,
    Media,
    #[serde(rename = "seed-catalog")]
    SeedCatalog,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub title: String,
    pub publisher: String,
    pub kind: SourceKind,
    pub accessed_at: NaiveDate,
    #[serde(default)]
    pub quoted_fact: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Editorial {
    #[serde(default)]
    pub tone: Option<String>,
    #[serde(default)]
    pub audience: Option<String>,
    #[serde(default)]
    pub spoiler: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub id: String,
    pub title: String,
    pub primary_genre: String,
    pub genres: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub summary: String,
    pub claim: String,
    #[serde(default)]
    pub explanation: Option<String>,
    pub sources: Vec<Source>,
    pub status: FactStatus,
    pub created_at: NaiveDate,
    pub updated_at: NaiveDate,
    pub revision: u32,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub duplicate_of: Option<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default = "default_canonical")]
    pub canonical: bool,
    #[serde(default)]
    pub importance: Option<f32>,
    #[serde(default)]
    pub editorial: Option<Editorial>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyEntry {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taxonomy {
    #[serde(default)]
    pub genres: BTreeMap<String, TaxonomyEntry>,
    #[serde(default)]
    pub tags: BTreeMap<String, TaxonomyEntry>,
}

const fn default_canonical() -> bool {
    true
}
