#![allow(dead_code)]

use anyhow::{bail, Result};
use core::panic;
use pgrx::PostgresType;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Bound};
use tantivy::{
    query::{
        AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, EmptyQuery,
        FastFieldRangeWeight, FuzzyTermQuery, MoreLikeThisQuery, PhrasePrefixQuery, PhraseQuery,
        Query, QueryParser, RangeQuery, RegexQuery, TermQuery, TermSetQuery,
    },
    query_grammar::Occur,
    schema::{Field, FieldType, IndexRecordOption, Value},
    Term,
};
use thiserror::Error;

#[derive(Debug, PostgresType, Deserialize, Serialize, Clone, PartialEq, Default)]
pub enum SearchQueryInput {
    All,
    Boolean {
        must: Vec<SearchQueryInput>,
        should: Vec<SearchQueryInput>,
        must_not: Vec<SearchQueryInput>,
    },
    Boost {
        query: Box<SearchQueryInput>,
        boost: f32,
    },
    ConstScore {
        query: Box<SearchQueryInput>,
        score: f32,
    },
    DisjunctionMax {
        disjuncts: Vec<SearchQueryInput>,
        tie_breaker: Option<f32>,
    },
    #[default]
    Empty,
    FastFieldRangeWeight {
        field: String,
        lower_bound: std::ops::Bound<u64>,
        upper_bound: std::ops::Bound<u64>,
    },
    FuzzyTerm {
        field: String,
        value: String,
        distance: Option<u8>,
        tranposition_cost_one: Option<bool>,
        prefix: Option<bool>,
    },
    MoreLikeThis {
        min_doc_frequency: Option<u64>,
        max_doc_frequency: Option<u64>,
        min_term_frequency: Option<usize>,
        max_query_terms: Option<usize>,
        min_word_length: Option<usize>,
        max_word_length: Option<usize>,
        boost_factor: Option<f32>,
        stop_words: Option<Vec<String>>,
        fields: Vec<(String, tantivy::schema::Value)>,
    },
    Parse {
        query_string: String,
    },
    Phrase {
        field: String,
        phrases: Vec<String>,
        slop: Option<u32>,
    },
    PhrasePrefix {
        field: String,
        phrases: Vec<String>,
        max_expansions: Option<u32>,
    },
    Range {
        field: String,
        lower_bound: std::ops::Bound<tantivy::schema::Value>,
        upper_bound: std::ops::Bound<tantivy::schema::Value>,
    },
    Regex {
        field: String,
        pattern: String,
    },
    Term {
        field: Option<String>,
        value: tantivy::schema::Value,
    },
    TermSet {
        terms: Vec<(String, tantivy::schema::Value)>,
    },
}

pub trait AsFieldType<T> {
    fn fields(&self) -> Vec<(FieldType, Field)>;

    fn as_field_type(&self, from: &T) -> Option<(FieldType, Field)>;

    fn is_field_type(&self, from: &T, value: &Value) -> bool {
        matches!(
            (self.as_field_type(from), value),
            (Some((FieldType::Str(_), _)), Value::Str(_))
                | (Some((FieldType::U64(_), _)), Value::U64(_))
                | (Some((FieldType::I64(_), _)), Value::I64(_))
                | (Some((FieldType::F64(_), _)), Value::F64(_))
                | (Some((FieldType::Bool(_), _)), Value::Bool(_))
                | (Some((FieldType::Date(_), _)), Value::Date(_))
                | (Some((FieldType::Facet(_), _)), Value::Facet(_))
                | (Some((FieldType::Bytes(_), _)), Value::Bytes(_))
                | (Some((FieldType::JsonObject(_), _)), Value::JsonObject(_))
                | (Some((FieldType::IpAddr(_), _)), Value::IpAddr(_))
        )
    }

    fn as_str(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::Str(_) => Some(field),
            _ => None,
        })
    }
    fn as_u64(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::U64(_) => Some(field),
            _ => None,
        })
    }
    fn as_i64(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::I64(_) => Some(field),
            _ => None,
        })
    }
    fn as_f64(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::F64(_) => Some(field),
            _ => None,
        })
    }
    fn as_bool(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::Bool(_) => Some(field),
            _ => None,
        })
    }
    fn as_date(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::Date(_) => Some(field),
            _ => None,
        })
    }
    fn as_facet(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::Facet(_) => Some(field),
            _ => None,
        })
    }
    fn as_bytes(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::Bytes(_) => Some(field),
            _ => None,
        })
    }
    fn as_json_object(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::JsonObject(_) => Some(field),
            _ => None,
        })
    }
    fn as_ip_addr(&self, from: &T) -> Option<Field> {
        self.as_field_type(from).and_then(|(ft, field)| match ft {
            FieldType::IpAddr(_) => Some(field),
            _ => None,
        })
    }
}

impl SearchQueryInput {
    pub fn into_tantivy_query(
        self,
        field_lookup: &impl AsFieldType<String>,
        parser: &mut QueryParser,
    ) -> Result<Box<dyn Query>> {
        match self {
            Self::All => Ok(Box::new(AllQuery)),
            Self::Boolean {
                must,
                should,
                must_not,
            } => {
                let mut subqueries = vec![];
                for input in must {
                    subqueries.push((Occur::Must, input.into_tantivy_query(field_lookup, parser)?));
                }
                for input in should {
                    subqueries.push((
                        Occur::Should,
                        input.into_tantivy_query(field_lookup, parser)?,
                    ));
                }
                for input in must_not {
                    subqueries.push((
                        Occur::MustNot,
                        input.into_tantivy_query(field_lookup, parser)?,
                    ));
                }
                Ok(Box::new(BooleanQuery::new(subqueries)))
            }
            Self::Boost { query, boost } => Ok(Box::new(BoostQuery::new(
                query.into_tantivy_query(field_lookup, parser)?,
                boost,
            ))),
            Self::ConstScore { query, score } => Ok(Box::new(ConstScoreQuery::new(
                query.into_tantivy_query(field_lookup, parser)?,
                score,
            ))),
            Self::DisjunctionMax {
                disjuncts,
                tie_breaker,
            } => {
                let disjuncts = disjuncts
                    .into_iter()
                    .map(|query| query.into_tantivy_query(field_lookup, parser))
                    .collect::<Result<_, _>>()?;
                if let Some(tie_breaker) = tie_breaker {
                    Ok(Box::new(DisjunctionMaxQuery::with_tie_breaker(
                        disjuncts,
                        tie_breaker,
                    )))
                } else {
                    Ok(Box::new(DisjunctionMaxQuery::new(disjuncts)))
                }
            }
            Self::Empty => Ok(Box::new(EmptyQuery)),
            Self::FastFieldRangeWeight {
                field,
                lower_bound,
                upper_bound,
            } => {
                field_lookup
                    .as_u64(&field)
                    .or_else(|| field_lookup.as_i64(&field))
                    .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?;

                Ok(Box::new(FastFieldRangeWeight::new(
                    field,
                    lower_bound,
                    upper_bound,
                )))
            }
            Self::FuzzyTerm {
                field,
                value,
                distance,
                tranposition_cost_one,
                prefix,
            } => {
                let field = field_lookup
                    .as_str(&field)
                    .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?;

                let term = Term::from_field_text(field, &value);
                let distance = distance.unwrap_or(1);
                let tranposition_cost_one = tranposition_cost_one.unwrap_or(false);
                if prefix.unwrap_or(false) {
                    Ok(Box::new(FuzzyTermQuery::new(
                        term,
                        distance,
                        tranposition_cost_one,
                    )))
                } else {
                    Ok(Box::new(FuzzyTermQuery::new_prefix(
                        term,
                        distance,
                        tranposition_cost_one,
                    )))
                }
            }
            Self::MoreLikeThis {
                min_doc_frequency,
                max_doc_frequency,
                min_term_frequency,
                max_query_terms,
                min_word_length,
                max_word_length,
                boost_factor,
                stop_words,
                fields,
            } => {
                let mut builder = MoreLikeThisQuery::builder();

                if let Some(min_doc_frequency) = min_doc_frequency {
                    builder = builder.with_min_doc_frequency(min_doc_frequency);
                }
                if let Some(max_doc_frequency) = max_doc_frequency {
                    builder = builder.with_max_doc_frequency(max_doc_frequency);
                }
                if let Some(min_term_frequency) = min_term_frequency {
                    builder = builder.with_min_term_frequency(min_term_frequency);
                }
                if let Some(max_query_terms) = max_query_terms {
                    builder = builder.with_max_query_terms(max_query_terms);
                }
                if let Some(min_work_length) = min_word_length {
                    builder = builder.with_min_word_length(min_work_length);
                }
                if let Some(max_work_length) = max_word_length {
                    builder = builder.with_max_word_length(max_work_length);
                }
                if let Some(boost_factor) = boost_factor {
                    builder = builder.with_boost_factor(boost_factor);
                }
                if let Some(stop_words) = stop_words {
                    builder = builder.with_stop_words(stop_words);
                }

                let mut fields_map = HashMap::new();
                for (field_name, value) in fields {
                    if !field_lookup.is_field_type(&field_name, &value) {
                        bail!("{}", QueryError::WrongFieldType(field_name))
                    }

                    let (_, field) = field_lookup
                        .as_field_type(&field_name)
                        .ok_or_else(|| QueryError::WrongFieldType(field_name.clone()))?;

                    fields_map.entry(field).or_insert_with(std::vec::Vec::new);

                    if let Some(vec) = fields_map.get_mut(&field) {
                        vec.push(value)
                    }
                }

                Ok(Box::new(
                    builder.with_document_fields(fields_map.into_iter().collect()),
                ))
            }
            Self::PhrasePrefix {
                field,
                phrases,
                max_expansions,
            } => {
                let field = field_lookup
                    .as_str(&field)
                    .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?;
                let terms = phrases
                    .into_iter()
                    .map(|phrase| Term::from_field_text(field, &phrase));
                let mut query = PhrasePrefixQuery::new(terms.collect());
                if let Some(max_expansions) = max_expansions {
                    query.set_max_expansions(max_expansions)
                }
                Ok(Box::new(query))
            }
            Self::Parse { query_string } => {
                Ok(Box::new(parser.parse_query(&query_string).map_err(
                    |err| QueryError::ParseError(err, query_string),
                )?))
            }
            Self::Phrase {
                field,
                phrases,
                slop,
            } => {
                let field = field_lookup
                    .as_str(&field)
                    .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?;
                let terms = phrases
                    .into_iter()
                    .map(|phrase| Term::from_field_text(field, &phrase));
                let mut query = PhraseQuery::new(terms.collect());
                if let Some(slop) = slop {
                    query.set_slop(slop)
                }
                Ok(Box::new(query))
            }
            Self::Range {
                field,
                lower_bound,
                upper_bound,
            } => {
                let field_name = field;
                let (field_type, field) = field_lookup
                    .as_field_type(&field_name)
                    .ok_or_else(|| QueryError::WrongFieldType(field_name.clone()))?;

                let lower_bound = match lower_bound {
                    Bound::Included(value) => {
                        Bound::Included(value_to_term(field, value, &field_type)?)
                    }
                    Bound::Excluded(value) => {
                        Bound::Excluded(value_to_term(field, value, &field_type)?)
                    }
                    Bound::Unbounded => Bound::Unbounded,
                };

                let upper_bound = match upper_bound {
                    Bound::Included(value) => {
                        Bound::Included(value_to_term(field, value, &field_type)?)
                    }
                    Bound::Excluded(value) => {
                        Bound::Excluded(value_to_term(field, value, &field_type)?)
                    }
                    Bound::Unbounded => Bound::Unbounded,
                };

                Ok(Box::new(RangeQuery::new_term_bounds(
                    field_name,
                    field_type.value_type(),
                    &lower_bound,
                    &upper_bound,
                )))
            }
            Self::Regex { field, pattern } => Ok(Box::new(
                RegexQuery::from_pattern(
                    &pattern,
                    field_lookup
                        .as_str(&field)
                        .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?,
                )
                .map_err(|err| QueryError::RegexError(err, pattern.clone()))?,
            )),
            Self::Term { field, value } => {
                let record_option = IndexRecordOption::WithFreqsAndPositions;
                if let Some(field) = field {
                    let (field_type, field) = field_lookup
                        .as_field_type(&field)
                        .ok_or_else(|| QueryError::NonIndexedField(field))?;
                    let term = value_to_term(field, value, &field_type)?;
                    Ok(Box::new(TermQuery::new(term, record_option)))
                } else {
                    // If no field is passed, then search all fields.
                    let all_fields = field_lookup.fields();
                    let mut terms = vec![];
                    for (field_type, field) in all_fields {
                        if let Ok(term) = value_to_term(field, value.clone(), &field_type) {
                            terms.push(term);
                        }
                    }

                    Ok(Box::new(TermSetQuery::new(terms)))
                }
            }
            Self::TermSet { terms: fields } => {
                let mut terms = vec![];
                for (field_name, field_value) in fields {
                    let (field_type, field) = field_lookup
                        .as_field_type(&field_name)
                        .ok_or_else(|| QueryError::NonIndexedField(field_name))?;
                    terms.push(value_to_term(field, field_value, &field_type)?);
                }

                Ok(Box::new(TermSetQuery::new(terms)))
            }
        }
    }
}

fn value_to_term(field: Field, value: Value, field_type: &FieldType) -> Result<Term> {
    Ok(match value {
        Value::Str(text) => {
            match field_type {
                FieldType::Date(_) => {
                    // Serialization turns date into string, so we have to turn it back into a Tantivy date
                    // First try with no precision beyond seconds, then try with precision
                    let datetime =
                        match chrono::NaiveDateTime::parse_from_str(&text, "%Y-%m-%dT%H:%M:%SZ") {
                            Ok(dt) => dt,
                            Err(_) => chrono::NaiveDateTime::parse_from_str(
                                &text,
                                "%Y-%m-%dT%H:%M:%S%.fZ",
                            )
                            .map_err(|_| QueryError::FieldTypeMismatch)?,
                        };
                    let tantivy_datetime = tantivy::DateTime::from_timestamp_micros(
                        datetime.and_utc().timestamp_micros(),
                    );
                    Term::from_field_date(field, tantivy_datetime)
                }
                _ => Term::from_field_text(field, &text),
            }
        }
        Value::PreTokStr(_) => panic!("pre-tokenized text cannot be converted to term"),
        Value::U64(u64) => {
            // Positive numbers seem to be automatically turned into u64s even if they are i64s,
            //     so we should use the field type to assign the term type
            match field_type {
                FieldType::I64(_) => Term::from_field_i64(field, u64 as i64),
                FieldType::U64(_) => Term::from_field_u64(field, u64),
                _ => panic!("invalid field type for u64 value"),
            }
        }
        Value::I64(i64) => Term::from_field_i64(field, i64),
        Value::F64(f64) => Term::from_field_f64(field, f64),
        Value::Bool(bool) => Term::from_field_bool(field, bool),
        Value::Date(date) => Term::from_field_date(field, date),
        Value::Facet(facet) => Term::from_facet(field, &facet),
        Value::Bytes(bytes) => Term::from_field_bytes(field, &bytes),
        Value::JsonObject(_) => panic!("json cannot be converted to term"),
        Value::IpAddr(ip) => Term::from_field_ip_addr(field, ip),
    })
}

#[derive(Debug, Error)]
enum QueryError {
    #[error("wrong field type for field: {0}")]
    WrongFieldType(String),
    #[error("invalid field map json: {0}")]
    FieldMapJsonValue(#[source] serde_json::Error),
    #[error("field map json must be an object")]
    FieldMapJsonObject,
    #[error("field '{0}' is not part of the pg_search index")]
    NonIndexedField(String),
    #[error("wrong type given for field")]
    FieldTypeMismatch,
    #[error("could not build regex with pattern '{1}': {0}")]
    RegexError(#[source] tantivy::TantivyError, String),
    #[error(
        r#"could not parse query string '{1}'.
           make sure to use column:term pairs, and to capitalize AND/OR."#
    )]
    ParseError(#[source] tantivy::query::QueryParserError, String),
}
