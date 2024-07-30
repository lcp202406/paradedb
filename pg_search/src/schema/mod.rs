// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod config;
mod document;

use anyhow::{Context, Result};
pub use config::*;
use derive_more::{AsRef, Display, From, Into};
pub use document::*;
use pgrx::{PgBuiltInOids, PgOid};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tantivy::schema::{
    DateOptions, Field, IndexRecordOption, JsonObjectOptions, NumericOptions, Schema,
    TextFieldIndexing, TextOptions, FAST, INDEXED, STORED,
};
use thiserror::Error;
use tokenizers::{SearchNormalizer, SearchTokenizer};

use crate::query::AsFieldType;

/// The id of a field, stored in the index.
#[derive(Debug, Clone, Display, From, AsRef, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[from(forward)]
pub struct SearchFieldName(pub String);

/// The name of a field, as it appears to Postgres.
#[derive(Debug, Copy, Clone, From, PartialEq, Eq, Serialize, Deserialize)]
#[from(forward)]
pub struct SearchFieldId(pub Field);

/// The name of the index, as it appears to Postgres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchIndexName(pub String);
/// The type of the search field.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFieldType {
    Text,
    I64,
    F64,
    U64,
    Bool,
    Json,
    Date,
}

impl TryFrom<&PgOid> for SearchFieldType {
    type Error = SearchIndexSchemaError;
    fn try_from(pg_oid: &PgOid) -> Result<Self, Self::Error> {
        match &pg_oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::UUIDOID => {
                    Ok(SearchFieldType::Text)
                }
                PgBuiltInOids::INT2OID | PgBuiltInOids::INT4OID | PgBuiltInOids::INT8OID => {
                    Ok(SearchFieldType::I64)
                }
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => Ok(SearchFieldType::U64),
                PgBuiltInOids::FLOAT4OID | PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                    Ok(SearchFieldType::F64)
                }
                PgBuiltInOids::BOOLOID => Ok(SearchFieldType::Bool),
                PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => Ok(SearchFieldType::Json),
                PgBuiltInOids::DATEOID
                | PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | PgBuiltInOids::TIMEOID
                | PgBuiltInOids::TIMETZOID => Ok(SearchFieldType::Date),
                _ => Err(SearchIndexSchemaError::InvalidPgOid(*pg_oid)),
            },
            _ => Err(SearchIndexSchemaError::InvalidPgOid(*pg_oid)),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, utoipa::ToSchema, PartialEq, Eq)]
pub enum SearchFieldConfig {
    Text {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default)]
        fast: bool,
        #[serde(default = "default_as_true")]
        stored: bool,
        #[serde(default = "default_as_true")]
        fieldnorms: bool,
        #[serde(default)]
        tokenizer: SearchTokenizer,
        #[schema(value_type = IndexRecordOptionSchema)]
        #[serde(default = "default_as_freqs_and_positions")]
        record: IndexRecordOption,
        #[serde(default)]
        normalizer: SearchNormalizer,
    },
    Json {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default)]
        fast: bool,
        #[serde(default = "default_as_true")]
        stored: bool,
        #[serde(default = "default_as_true")]
        expand_dots: bool,
        #[serde(default)]
        tokenizer: SearchTokenizer,
        #[schema(value_type = IndexRecordOptionSchema)]
        #[serde(default = "default_as_freqs_and_positions")]
        record: IndexRecordOption,
        #[serde(default)]
        normalizer: SearchNormalizer,
    },
    Numeric {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
        #[serde(default = "default_as_true")]
        stored: bool,
    },
    Boolean {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
        #[serde(default = "default_as_true")]
        stored: bool,
    },
    Date {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
        #[serde(default = "default_as_true")]
        stored: bool,
    },
    Ctid,
}

impl SearchFieldConfig {
    pub fn text_from_json(value: serde_json::Value) -> Result<Self> {
        let obj = value
            .as_object()
            .context("Expected a JSON object for Text configuration")?;

        let indexed = match obj.get("indexed") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'indexed' field should be a boolean")),
            None => Ok(true),
        }?;

        let fast = match obj.get("fast") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'fast' field should be a boolean")),
            None => Ok(false),
        }?;

        let stored = match obj.get("stored") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'stored' field should be a boolean")),
            None => Ok(true),
        }?;

        let fieldnorms = match obj.get("fieldnorms") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'fieldnorms' field should be a boolean")),
            None => Ok(true),
        }?;

        let tokenizer = match obj.get("tokenizer") {
            Some(v) => SearchTokenizer::from_json_value(v),
            None => Ok(SearchTokenizer::Default),
        }?;

        let record = match obj.get("record") {
            Some(v) => serde_json::from_value(v.clone()),
            None => Ok(default_as_freqs_and_positions()),
        }?;

        let normalizer = match obj.get("normalizer") {
            Some(v) => serde_json::from_value(v.clone()),
            None => Ok(SearchNormalizer::Raw),
        }?;

        Ok(SearchFieldConfig::Text {
            indexed,
            fast,
            stored,
            fieldnorms,
            tokenizer,
            record,
            normalizer,
        })
    }

    pub fn json_from_json(value: serde_json::Value) -> Result<Self> {
        let obj = value
            .as_object()
            .context("Expected a JSON object for Json configuration")?;

        let indexed = match obj.get("indexed") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'indexed' field should be a boolean")),
            None => Ok(true),
        }?;

        let fast = match obj.get("fast") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'fast' field should be a boolean")),
            None => Ok(false),
        }?;

        let stored = match obj.get("stored") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'stored' field should be a boolean")),
            None => Ok(true),
        }?;

        let expand_dots = match obj.get("expand_dots") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'expand_dots' field should be a boolean")),
            None => Ok(true),
        }?;

        let tokenizer = match obj.get("tokenizer") {
            Some(v) => SearchTokenizer::from_json_value(v),
            None => Ok(SearchTokenizer::Default),
        }?;

        let record = match obj.get("record") {
            Some(v) => serde_json::from_value(v.clone()),
            None => Ok(default_as_freqs_and_positions()),
        }?;

        let normalizer = match obj.get("normalizer") {
            Some(v) => serde_json::from_value(v.clone()),
            None => Ok(SearchNormalizer::Raw),
        }?;

        Ok(SearchFieldConfig::Json {
            indexed,
            fast,
            stored,
            expand_dots,
            tokenizer,
            record,
            normalizer,
        })
    }

    pub fn numeric_from_json(value: serde_json::Value) -> Result<Self> {
        let obj = value
            .as_object()
            .context("Expected a JSON object for Numeric configuration")?;

        let indexed = match obj.get("indexed") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'indexed' field should be a boolean")),
            None => Ok(true),
        }?;

        let fast = match obj.get("fast") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'fast' field should be a boolean")),
            None => Ok(true),
        }?;

        let stored = match obj.get("stored") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'stored' field should be a boolean")),
            None => Ok(true),
        }?;

        Ok(SearchFieldConfig::Numeric {
            indexed,
            fast,
            stored,
        })
    }

    pub fn boolean_from_json(value: serde_json::Value) -> Result<Self> {
        let obj = value
            .as_object()
            .context("Expected a JSON object for Boolean configuration")?;

        let indexed = match obj.get("indexed") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'indexed' field should be a boolean")),
            None => Ok(true),
        }?;

        let fast = match obj.get("fast") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'fast' field should be a boolean")),
            None => Ok(true),
        }?;

        let stored = match obj.get("stored") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'stored' field should be a boolean")),
            None => Ok(true),
        }?;

        Ok(SearchFieldConfig::Boolean {
            indexed,
            fast,
            stored,
        })
    }

    pub fn date_from_json(value: serde_json::Value) -> Result<Self> {
        let obj = value
            .as_object()
            .context("Expected a JSON object for Date configuration")?;

        let indexed = match obj.get("indexed") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'indexed' field should be a boolean")),
            None => Ok(true),
        }?;

        let fast = match obj.get("fast") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'fast' field should be a boolean")),
            None => Ok(true),
        }?;

        let stored = match obj.get("stored") {
            Some(v) => v
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'stored' field should be a boolean")),
            None => Ok(true),
        }?;

        Ok(SearchFieldConfig::Date {
            indexed,
            fast,
            stored,
        })
    }
}

impl SearchFieldConfig {
    pub fn from_json(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    pub fn default_text() -> Self {
        Self::from_json(json!({"Text": {}}))
    }

    pub fn default_numeric() -> Self {
        Self::from_json(json!({"Numeric": {}}))
    }

    pub fn default_boolean() -> Self {
        Self::from_json(json!({"Boolean": {}}))
    }

    pub fn default_json() -> Self {
        Self::from_json(json!({"Json": {}}))
    }

    pub fn default_date() -> Self {
        Self::from_json(json!({"Date": {}}))
    }
}

impl From<SearchFieldConfig> for TextOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut text_options = TextOptions::default();
        match config {
            SearchFieldConfig::Text {
                indexed,
                fast,
                stored,
                fieldnorms,
                tokenizer,
                record,
                normalizer,
            } => {
                if stored {
                    text_options = text_options.set_stored();
                }
                if fast {
                    text_options = text_options.set_fast(Some(normalizer.name()));
                }
                if indexed {
                    let text_field_indexing = TextFieldIndexing::default()
                        .set_index_option(record)
                        .set_fieldnorms(fieldnorms)
                        .set_tokenizer(&tokenizer.name());

                    text_options = text_options.set_indexing_options(text_field_indexing);
                }
            }
            _ => panic!("attemped to convert non-text search field config to tantivy text config"),
        }
        text_options
    }
}

impl From<SearchFieldConfig> for NumericOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut numeric_options = NumericOptions::default();
        match config {
            SearchFieldConfig::Numeric {
                indexed,
                fast,
                stored,
            }
            // Following the example of Quickwit, which uses NumericOptions for boolean options.
            | SearchFieldConfig::Boolean { indexed, fast, stored } => {
                if stored {
                    numeric_options = numeric_options.set_stored();
                }
                if fast {
                    numeric_options = numeric_options.set_fast();
                }
                if indexed {
                    numeric_options = numeric_options.set_indexed();
                }
            }
            _ => {
                panic!(
                    "attemped to convert non-numeric search field config to tantivy numeric config"
                )
            }
        }
        numeric_options
    }
}

impl From<SearchFieldConfig> for JsonObjectOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut json_options = JsonObjectOptions::default();
        match config {
            SearchFieldConfig::Json {
                indexed,
                fast,
                stored,
                expand_dots,
                tokenizer,
                record,
                normalizer,
            } => {
                if stored {
                    json_options = json_options.set_stored();
                }
                if fast {
                    json_options = json_options.set_fast(Some(normalizer.name()));
                }
                if expand_dots {
                    json_options = json_options.set_expand_dots_enabled();
                }
                if indexed {
                    let text_field_indexing = TextFieldIndexing::default()
                        .set_index_option(record)
                        .set_tokenizer(&tokenizer.name());

                    json_options = json_options.set_indexing_options(text_field_indexing);
                }
            }
            _ => {
                panic!("attemped to convert non-json search field config to tantivy json config")
            }
        }

        json_options
    }
}

impl From<SearchFieldConfig> for DateOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut date_options = DateOptions::default();
        match config {
            SearchFieldConfig::Date {
                indexed,
                fast,
                stored,
            } => {
                if stored {
                    date_options = date_options.set_stored();
                }
                if fast {
                    date_options = date_options.set_fast();
                }
                if indexed {
                    date_options = date_options.set_indexed();
                }
            }
            _ => {
                panic!("attemped to convert non-date search field config to tantivy date config")
            }
        }
        date_options
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchField {
    /// The id of the field, stored in the index.
    pub id: SearchFieldId,
    /// The name of the field, as it appears to Postgres.
    pub name: SearchFieldName,
    /// Configuration for the field passed at index build time.
    pub config: SearchFieldConfig,
    /// Field type
    pub type_: SearchFieldType,
}

impl From<&SearchField> for Field {
    fn from(val: &SearchField) -> Self {
        val.id.0
    }
}

#[derive(Serialize, Deserialize, Clone, Into)]
pub struct SearchIndexSchema {
    /// The fields that are stored in the index.
    pub fields: Vec<SearchField>,
    /// The index of the key field in the fields vector.
    pub key: usize,
    /// The index of the ctid field in the fields vector.
    pub ctid: usize,
    /// The underlying tantivy schema
    #[into]
    pub schema: Schema,
    /// A lookup cache for retrieving search fields.
    #[serde(skip_serializing)]
    pub lookup: Option<HashMap<SearchFieldName, usize>>,
}

impl SearchIndexSchema {
    pub fn new(
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_index: usize,
    ) -> Result<Self, SearchIndexSchemaError> {
        let mut builder = Schema::builder();
        let mut search_fields = vec![];

        let mut ctid_index = 0;
        for (index, (name, config, field_type)) in fields.into_iter().enumerate() {
            if config == SearchFieldConfig::Ctid {
                ctid_index = index
            }

            let id: SearchFieldId = match &config {
                SearchFieldConfig::Ctid => {
                    builder.add_u64_field(name.as_ref(), INDEXED | STORED | FAST)
                }
                _ => match field_type {
                    SearchFieldType::Text => builder.add_text_field(name.as_ref(), config.clone()),
                    SearchFieldType::I64 => builder.add_i64_field(name.as_ref(), config.clone()),
                    SearchFieldType::U64 => builder.add_u64_field(name.as_ref(), config.clone()),
                    SearchFieldType::F64 => builder.add_f64_field(name.as_ref(), config.clone()),
                    SearchFieldType::Bool => builder.add_bool_field(name.as_ref(), config.clone()),
                    SearchFieldType::Json => builder.add_json_field(name.as_ref(), config.clone()),
                    SearchFieldType::Date => builder.add_date_field(name.as_ref(), config.clone()),
                },
            }
            .into();

            search_fields.push(SearchField {
                id,
                name,
                config,
                type_: field_type,
            });
        }

        let schema = builder.build();

        Ok(Self {
            key: key_index,
            ctid: ctid_index,
            schema,
            lookup: Self::build_lookup(&search_fields).into(),
            fields: search_fields,
        })
    }

    fn build_lookup(search_fields: &[SearchField]) -> HashMap<SearchFieldName, usize> {
        let mut lookup = HashMap::new();
        search_fields
            .iter()
            .enumerate()
            .for_each(|(idx, search_field)| {
                let name = search_field.name.clone();
                lookup.insert(name, idx);
            });
        lookup
    }

    pub fn ctid_field(&self) -> SearchField {
        self.fields
            .get(self.ctid)
            .expect("ctid field should be present on search schema")
            .clone()
    }

    pub fn key_field(&self) -> SearchField {
        self.fields
            .get(self.key)
            .expect("key field should be present on search schema")
            .clone()
    }

    pub fn new_document(&self) -> SearchDocument {
        let doc = tantivy::TantivyDocument::new();
        let key = self.key_field().id;
        let ctid = self.ctid_field().id;
        SearchDocument { doc, key, ctid }
    }

    pub fn get_search_field(&self, name: &SearchFieldName) -> Option<&SearchField> {
        if let Some(lookup) = &self.lookup {
            lookup.get(name).and_then(|idx| self.fields.get(*idx))
        } else {
            let lookup = Self::build_lookup(&self.fields);
            lookup.get(name).and_then(|idx| self.fields.get(*idx))
        }
    }
}

// Index record schema
#[allow(unused)] // used by serde
#[derive(utoipa::ToSchema)]
pub enum IndexRecordOptionSchema {
    #[schema(rename = "basic")]
    Basic,
    #[schema(rename = "freq")]
    WithFreqs,
    #[schema(rename = "position")]
    WithFreqsAndPositions,
}

pub trait ToString {
    fn to_string(&self) -> String;
}

impl ToString for IndexRecordOption {
    fn to_string(&self) -> String {
        match self {
            IndexRecordOption::Basic => "basic".to_string(),
            IndexRecordOption::WithFreqs => "freq".to_string(),
            IndexRecordOption::WithFreqsAndPositions => "position".to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum SearchIndexSchemaError {
    #[error("invalid field type for numeric: {0:?}")]
    InvalidNumericType(SearchFieldType),
    #[error("invalid postgres oid passed to search index schema: {0:?}")]
    InvalidPgOid(PgOid),
    #[error("no key field specified for search index")]
    NoKeyFieldSpecified,
    #[error("no ctid field specified for search index")]
    NoCtidFieldSpecified,
}

fn default_as_true() -> bool {
    true
}

fn default_as_freqs_and_positions() -> IndexRecordOption {
    IndexRecordOption::WithFreqsAndPositions
}

impl AsFieldType<String> for SearchIndexSchema {
    fn fields(&self) -> Vec<(tantivy::schema::FieldType, Field)> {
        self.fields
            .iter()
            .map(|search_field| {
                let field = search_field.id.0;
                let field_type = self.schema.get_field_entry(field).field_type().clone();
                (field_type, field)
            })
            .collect()
    }
    fn as_field_type(&self, from: &String) -> Option<(tantivy::schema::FieldType, Field)> {
        self.get_search_field(&SearchFieldName(from.into()))
            .map(|search_field| {
                let field = search_field.id.0;
                let field_type = self.schema.get_field_entry(field).field_type().clone();
                (field_type, field)
            })
    }
}
