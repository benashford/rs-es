/*
 * Copyright 2016-2019 Ben Ashford
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Implementation of ElasticSearch Mapping operation

//!
//! Please note: this will grow and become a full implementation of the ElasticSearch
//! [Indices API](https://www.elastic.co/guide/en/elasticsearch/reference/current/indices.html)
//! so subtle (potentially breaking) changes will be made to the API when that happens

use std::collections::HashMap;
use std::hash::Hash;

use reqwest::StatusCode;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::{error::EsError, operations::GenericResult, Client, EsResponse};

pub type DocType<'a> = HashMap<&'a str, HashMap<&'a str, &'a str>>;
pub type Mapping<'a> = HashMap<&'a str, DocType<'a>>;

#[derive(Debug, Serialize)]
pub struct Settings {
    pub number_of_shards: u32,
    pub analysis: Analysis,
}

#[derive(Debug, Serialize)]
pub struct Analysis {
    pub filter: Map<String, Value>,
    pub analyzer: Map<String, Value>,
}

/// An indexing operation
#[derive(Debug)]
pub struct MappingOperation<'a, 'b> {
    /// The HTTP client that this operation will use
    client: &'a mut Client,

    /// The index that will be created and eventually mapped
    index: &'b str,

    /// A map containing the doc types and their mapping
    mapping: Option<&'b Mapping<'b>>,

    /// A struct reflecting the settings that enable the
    /// customization of analyzers
    settings: Option<&'b Settings>,
}

impl<'a, 'b> MappingOperation<'a, 'b> {
    pub fn new(client: &'a mut Client, index: &'b str) -> MappingOperation<'a, 'b> {
        MappingOperation {
            client,
            index,
            mapping: None,
            settings: None,
        }
    }

    /// Set the actual mapping
    pub fn with_mapping(&'b mut self, mapping: &'b Mapping) -> &'b mut Self {
        self.mapping = Some(mapping);
        self
    }

    /// Set the settings
    pub fn with_settings(&'b mut self, settings: &'b Settings) -> &'b mut Self {
        self.settings = Some(settings);
        self
    }

    /// If settings have been provided, the index will be created with them. If the index already
    /// exists, an `Err(EsError)` will be returned.
    /// If mapping have been set too, the properties will be applied. The index will be unavailable
    /// during this process.
    /// Nothing will be done if either mapping and settings are not present.
    pub fn send(&'b mut self) -> Result<MappingResult, EsError> {
        // Return earlier if there is nothing to do
        if self.mapping.is_none() && self.settings.is_none() {
            return Ok(MappingResult);
        }

        let url = self.index.to_owned();

        if self.mapping.is_none() {
            let body = hashmap("settings", self.settings.unwrap());
            let _   = self.client.put_body_op(&url, &body)?;

            let _ = self.client.wait_for_status("yellow", "5s");
        }

        if let Some(mapping) = self.mapping {
            let _ = self.client.close_index(self.index);

            let mut mappings: HashMap<&str, Mapping> = HashMap::new();

            for (entity, properties) in mapping.into_iter() {
                let properties = hashmap("properties", properties.to_owned());
                mappings.insert(entity.to_owned(), properties.to_owned());
            }

            let body = match self.settings {
                Some(settings) => serde_json::json!({
                    "mappings": mappings,
                    "settings": settings
                }),
                None => serde_json::json!({
                    "mappings": mappings,
                })
            };

            let _ = self.client.put_body_op(&url, &body)?;

            let _ = self.client.open_index(self.index);
        }

        Ok(MappingResult)
    }
}

impl Client {
    /// Open the index, making it available.
    pub fn open_index<'a>(&'a mut self, index: &'a str) -> Result<GenericResult, EsError> {
        let url = format!("{}/_open", index);
        let response = self.post_op(&url)?;

        match response.status_code() {
            StatusCode::OK => Ok(response.read_response()?),
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }

    /// Close the index, making it unavailable and modifiable.
    pub fn close_index<'a>(&'a mut self, index: &'a str) -> Result<GenericResult, EsError> {
        let url = format!("{}/_close", index);
        let response = self.post_op(&url)?;

        match response.status_code() {
            StatusCode::OK => Ok(response.read_response()?),
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }

    /// TODO: Return proper health data from
    /// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-health.html
    pub fn wait_for_status<'a>(
        &'a mut self,
        status: &'a str,
        timeout: &'a str,
    ) -> Result<(), EsError> {
        let url = format!(
            "_cluster/health?wait_for_status={}&timeout={}",
            status, timeout
        );
        let response = self.get_op(&url)?;

        match response.status_code() {
            StatusCode::OK => Ok(()),
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }
}

/// The result of a mapping operation
#[derive(Debug)]
pub struct MappingResult;

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Debug, Serialize)]
    pub struct Author {
        pub name: String,
    }

    #[test]
    fn test_mapping() {
        let index_name = "tests_test_mapping";
        let mut client = crate::tests::make_client();

        // TODO - this fails in many cases (specifically on TravisCI), but we ignore the
        // failures anyway
        let _ = client.delete_index(index_name);

        let mapping = hashmap2(
            "post",
            hashmap2(
                "created_at",
                hashmap2("type", "date", "format", "date_time"),
                "title",
                hashmap2("type", "string", "index", "not_analyzed"),
            ),
            "author",
            hashmap("name", hashmap("type", "string")),
        );

        let settings = Settings {
            number_of_shards: 1,

            analysis: Analysis {
                filter: serde_json::json! ({
                    "autocomplete_filter": {
                        "type": "edge_ngram",
                        "min_gram": 1,
                        "max_gram": 2,
                    }
                })
                .as_object()
                .expect("by construction 'autocomplete_filter' should be a map")
                .clone(),
                analyzer: serde_json::json! ({
                    "autocomplete": {
                        "type": "custom",
                        "tokenizer": "standard",
                        "filter": [ "lowercase", "autocomplete_filter"]
                    }
                })
                .as_object()
                .expect("by construction 'autocomplete' should be a map")
                .clone(),
            },
        };

        // TODO add appropriate functions to the `Client` struct
        let result = MappingOperation::new(&mut client, index_name)
            .with_mapping(&mapping)
            .with_settings(&settings)
            .send();
        assert!(result.is_ok());

        {
            let result_wrapped = client
                .index(index_name, "post")
                .with_doc(&Author {
                    name: "Homu".to_owned(),
                })
                .send();

            assert!(result_wrapped.is_ok());

            let result = result_wrapped.unwrap();
            assert!(result.created);
        }
    }
}

fn hashmap<K, V>(k: K, v: V) -> HashMap<K, V>
where
    K: Eq + Hash,
{
    let mut m = HashMap::with_capacity(1);
    m.insert(k, v);
    m
}

#[allow(dead_code)]
fn hashmap2<K, V>(k1: K, v1: V, k2: K, v2: V) -> HashMap<K, V>
where
    K: Eq + Hash,
{
    let mut m = HashMap::with_capacity(2);
    m.insert(k1, v1);
    m.insert(k2, v2);
    m
}
