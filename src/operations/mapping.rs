/*
 * Copyright 2016 Ben Ashford
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

use serde_json::{Value, Map};

use hyper::status::StatusCode;

use ::{Client, EsResponse};
use ::error::EsError;
use ::operations::GenericResult;

#[derive(Serialize)]
pub struct Settings {
    pub number_of_shards: u32,
    pub analysis: Analysis
}

#[derive(Serialize)]
pub struct Analysis {
    pub filter:   Map<String, Value>,
    pub analyzer: Map<String, Value>
}

/// An indexing operation
pub struct MappingOperation<'a, 'b> {
    /// The HTTP client that this operation will use
    client:    &'a mut Client,

    /// The index that will be created and eventually mapped
    index:     &'b str,

    /// A map containing the doc types and their mapping
    mappings: Option<&'b Value>,

    /// A struct reflecting the settings that enable the
    /// customization of analyzers
    settings: Option<&'b Settings>
}

impl<'a, 'b> MappingOperation<'a, 'b> {
    pub fn new(client: &'a mut Client,
               index: &'b str) -> MappingOperation<'a, 'b> {
        MappingOperation {
            client:   client,
            index:    index,
            mappings: None,
            settings: None
        }
    }

    /// Set the actual mapping
    pub fn with_mappings(&'b mut self, mappings: &'b Value) -> &'b mut Self {
        self.mappings = Some(mappings);
        self
    }

    /// Set the settings
    pub fn with_settings(&'b mut self, settings: &'b Settings) -> &'b mut Self {
        self.settings = Some(settings);
        self
    }

    /// If settings have been provided, the index will be created with them. If the index already
    /// exists, an `Err(EsError)` will be returned.
    /// If mappings have been set too, the properties will be applied. The index will be unavailable
    /// during this process.
    /// Nothing will be done if either mappings and settings are not present.
    pub fn send(&'b mut self) -> Result<MappingResult, EsError> {
        // Return earlier if there is nothing to do
        if self.mappings.is_none() && self.settings.is_none() {
            return Ok(MappingResult);
        }

        let url = format!("{}", self.index);

        if self.mappings.is_none() {
            let body = hashmap! { "settings" => self.settings.unwrap() };
            let _   = self.client.put_body_op(&url, &body)?;

            let _ = self.client.wait_for_status("yellow", "5s");
        }

        if let Some(mappings) = self.mappings {
            let _ = self.client.close_index(self.index);

            let body = match self.settings {
                Some(settings) => json!({
                    "mappings": mappings,
                    "settings": settings
                }),
                None => json!({
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
            &StatusCode::Ok => Ok(response.read_response()?),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }

    /// Close the index, making it unavailable and modifiable.
    pub fn close_index<'a>(&'a mut self, index: &'a str) -> Result<GenericResult, EsError> {
        let url = format!("{}/_close", index);
        let response = self.post_op(&url)?;

        match response.status_code() {
            &StatusCode::Ok => Ok(response.read_response()?),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }

    /// TODO: Return proper health data from
    /// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-health.html
    pub fn wait_for_status<'a>(&'a mut self, status: &'a str, timeout: &'a str) -> Result<(), EsError> {
        let url = format!("_cluster/health?wait_for_status={}&timeout={}", status, timeout);
        let response = self.get_op(&url)?;

        match response.status_code() {
            &StatusCode::Ok => Ok(()),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }
}

/// The result of a mapping operation
#[derive(Debug)]
pub struct MappingResult;

#[cfg(test)]
pub mod tests {
    extern crate env_logger;

    use super::*;

    #[derive(Debug, Serialize)]
    pub struct Author {
        pub name: String
    }

    #[test]
    fn test_mappings() {
        let index_name = "tests_test_mappings";
        let mut client = ::tests::make_client();

        // TODO - this fails in many cases (specifically on TravisCI), but we ignore the
        // failures anyway
        let _ = client.delete_index(index_name);

        let mappings = json! ({
            "post": {
                "properties": {
                    "created_at": {
                        "type": "date",
                        "format": "date_time"
                    },
                    "title": {
                        "type": "string",
                        "index": "not_analyzed"
                    }
                }
            },
            "author": {
                "properties": {
                    "name": {
                        "type": "string"
                    }
                }
            }
        });

        let settings = Settings {
            number_of_shards: 1,

            analysis: Analysis {
                filter: json! ({
                    "autocomplete_filter": {
                        "type": "edge_ngram",
                        "min_gram": 1,
                        "max_gram": 2,
                    }
                }).as_object().expect("by construction 'autocomplete_filter' should be a map").clone(),
                analyzer: json! ({
                    "autocomplete": {
                        "type": "custom",
                        "tokenizer": "standard",
                        "filter": [ "lowercase", "autocomplete_filter"]
                    }
                }).as_object().expect("by construction 'autocomplete' should be a map").clone()
            }
        };

        // TODO add appropriate functions to the `Client` struct
        let result = MappingOperation::new(&mut client, index_name)
            .with_mappings(&mappings)
            .with_settings(&settings)
            .send();
        let _ = result.unwrap();

         {
            let result_wrapped = client
                .index(index_name, "post")
                .with_doc(&Author { name: "Homu".to_owned() })
                .send();

            assert!(result_wrapped.is_ok());

            let result = result_wrapped.unwrap();
            assert!(result.created);
        }
    }
}
