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

use std::collections::HashMap;

use ::Client;
use ::error::EsError;

pub type Properties = HashMap<String, HashMap<String, String>>;

/// An indexing operation
pub struct MappingOperation<'a, 'b> {
    /// The HTTP client that this operation will use
    client:     &'a mut Client,

    /// The index that will be created and eventually mapped
    index:      &'b str,

    /// The actual mapping
    properties: &'b Properties
}

impl<'a, 'b> MappingOperation<'a, 'b> {
    pub fn new(client: &'a mut Client, index: &'b str, properties: &'b Properties) -> MappingOperation<'a, 'b> {
        MappingOperation {
            client:     client,
            index:      index,
            properties: properties
        }
    }

    pub fn send(&'b mut self) -> Result<MappingResult, EsError> {
        let body = hashmap! {
            "mappings" => hashmap! {
                "sample" => hashmap! {
                    "properties" => self.properties
                }
            }
        };

        let url = format!("{}", self.index);
        let (_, _) = try!(self.client.put_body_op(&url, &body));
        Ok(MappingResult)
    }
}

/// The result of a mapping operation
#[derive(Debug)]
pub struct MappingResult;

#[cfg(test)]
pub mod tests {
    extern crate env_logger;

    use std::collections::HashMap;

    use super::MappingOperation;

    #[test]
    fn test_mapping() {
        let index_name = "tests_test_mapping";
        let mut client = ::tests::make_client();

        client.delete_op(&format!("/{}", index_name)).unwrap();

        let mapping = hashmap! {
            "created_at".to_owned() => hashmap! {
                "type".to_owned() => "date".to_owned(),
                "format".to_owned() => "epoch_second".to_owned()
            },

            "title".to_owned() => hashmap! {
                "type".to_owned() => "string".to_owned(),
                "index".to_owned() => "not_analyzed".to_owned()
            }
        };

        let result = MappingOperation::new(&mut client, index_name, &mapping).send();
        assert!(result.is_ok());
    }
}
