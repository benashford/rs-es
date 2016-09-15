/*
 * Copyright 2015-2016 Ben Ashford
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

//! Implementation of ElasticSearch Index managemenent operations of the
//! Indices API

use std::collections::HashMap;
use std::marker::PhantomData;

use hyper::status::StatusCode;

use serde_json::Value;

use ::{Client, EsResponse};
use ::error::EsError;
use ::operations::{format_multi, GenericResult};

#[derive(Default, Serialize)]
struct CreateIndexBody<'b> {
    // TODO - remove
    tmp: PhantomData<&'b str>
}

pub struct CreateIndexOperation<'a, 'b> {
    client: &'a mut Client,
    index: &'a str,
    body: CreateIndexBody<'b>
}

impl<'a, 'b> CreateIndexOperation<'a, 'b> {
    pub fn new(client: &'a mut Client, index: &'a str) -> Self {
        CreateIndexOperation {
            client: client,
            index:  index,
            body: Default::default()
        }
    }

    // TODO: replace with specific result
    pub fn send(&mut self) -> Result<GenericResult, EsError> {
        let url = format!("/{}/", self.index);
        let response = try!(self.client.put_body_op(&url, &self.body));
        match response.status_code() {
            &StatusCode::Ok => Ok(try!(response.read_response())),
            _ => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

impl Client {
    /// Create a specified index.
    ///
    /// https://www.elastic.co/guide/en/elasticsearch/reference/2.0/indices-create-index.html
    pub fn create_index<'a, 'b>(&'a mut self,
                                index: &'a str) -> CreateIndexOperation<'a, 'b> {
        CreateIndexOperation::new(self, index)
    }
}

impl Client {
    /// Delete given index
    ///
    /// TODO: ensure all options are supported, replace with a `DeleteIndexOperation` to
    /// follow the pattern defined elsewhere.
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/2.x/indices-delete-index.html
    pub fn delete_index<'a>(&'a mut self, index: &'a str) -> Result<GenericResult, EsError> {
        let url = format!("/{}/", index);
        let response = try!(self.delete_op(&url));

        match response.status_code() {
            &StatusCode::Ok => Ok(try!(response.read_response())),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }
}

pub struct GetIndexOperation<'a, 'b> {
    client: &'a mut Client,
    indexes: &'b [&'b str]
}

impl<'a, 'b> GetIndexOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> Self {
        GetIndexOperation {
            client: client,
            indexes: &[]
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn send(&mut self) -> Result<Option<GetIndexResult>, EsError> {
        let url = format!("/{}/", format_multi(self.indexes));
        let response = try!(self.client.get_op(&url));
        match response.status_code() {
            &StatusCode::Ok => Ok(Some(try!(response.read_response()))),
            &StatusCode::NotFound => Ok(None),
            _ => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

impl Client {
    /// Get a given index
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/2.0/indices-get-index.html
    pub fn get_index<'a, 'b>(&'a mut self) -> GetIndexOperation<'a, 'b> {
        GetIndexOperation::new(self)
    }
}

#[derive(Debug, Deserialize)]
pub struct IndexSettingsResultVersion {
    pub created: String
}

#[derive(Debug, Deserialize)]
pub struct IndexSettingsResult {
    pub creation_date: String,
    pub number_of_shards: u64,
    pub number_of_replicas: u64,
    pub uuid: String, // Not an actual UUID
    pub version: IndexSettingsResultVersion
}

#[derive(Debug, Deserialize)]
pub struct SettingsResult {
    pub index: IndexSettingsResult
}

pub type MappingResult = Value; // TODO - replace with specific type, may be the inverse
                                // of the mapping type needed to put mappings
pub type MappingsResult = HashMap<String, MappingResult>;

#[derive(Debug, Deserialize)]
pub struct IndexResult {
    pub aliases: Value, // TODO - replace with specific
    pub mappings: MappingsResult,
    pub settings: SettingsResult
}

pub type GetIndexResult = HashMap<String, IndexResult>;

#[cfg(test)]
pub mod tests {
    use ::tests::{clean_db, TestDocument, make_client};

    #[test]
    fn test_create_index() {
        let index_name = "test_create_index";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        {
            let result = client.create_index(index_name).send();
            assert!(result.is_ok());
        }
        {
            let result = client.get_index().with_indexes(&[index_name]).send();
            let res = result.unwrap().unwrap();
            println!("Get index result: {:?}", res);
            assert!(false);
        }
    }

    #[test]
    fn test_delete_index() {
        let index_name = "test_delete_index";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        {
            let result = client
                .index(index_name, "test_type")
                .with_doc(&TestDocument::new().with_int_field(1))
                .send();
            assert!(result.is_ok());
        }
        {
            let result = client.delete_index(index_name);
            info!("DELETE INDEX RESULT: {:?}", result);

            assert!(result.is_ok());

            let result_wrapped = result.unwrap();
            assert!(result_wrapped.acknowledged);
        }
    }

    #[test]
    fn test_get_index() {
        let index_name = "test_get_index";
        let mut client = make_client();
        client.delete_index(index_name); // deliberately not unwrapping
        {
            let result = client.get_index().with_indexes(&[index_name]).send();
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }
        {
            client.index(index_name, "test_type")
                .with_doc(&TestDocument::new().with_int_field(2))
                .send()
                .unwrap();
            let get_result = client.get_index().with_indexes(&[index_name]).send();
            assert!(get_result.is_ok());

            let get_opt = get_result.unwrap();
            assert!(get_opt.is_some());

            let res = get_opt.unwrap();
            println!("Get index result: {:?}", res);

            let this_index_res = res.get(index_name);
            assert!(this_index_res.is_some());
        }
    }
}
