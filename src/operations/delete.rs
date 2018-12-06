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

//! Implementation of delete operations, both Delete-By-Query and Delete-By-Id

use reqwest::StatusCode;

use super::common::{OptionVal, Options};
use error::EsError;
use {Client, EsResponse};

#[derive(Debug)]
pub struct DeleteOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The index
    index: &'b str,

    /// The type
    doc_type: &'b str,

    /// The ID
    id: &'b str,

    /// Optional options
    options: Options<'b>,
}

impl<'a, 'b> DeleteOperation<'a, 'b> {
    pub fn new(
        client: &'a mut Client,
        index: &'b str,
        doc_type: &'b str,
        id: &'b str,
    ) -> DeleteOperation<'a, 'b> {
        DeleteOperation {
            client: client,
            index: index,
            doc_type: doc_type,
            id: id,
            options: Options::new(),
        }
    }

    add_option!(with_version, "version");
    add_option!(with_version_type, "version_type");
    add_option!(with_routing, "routing");
    add_option!(with_parent, "parent");
    add_option!(with_consistency, "consistency");
    add_option!(with_refresh, "refresh");
    add_option!(with_timeout, "timeout");

    pub fn send(&'a mut self) -> Result<DeleteResult, EsError> {
        let url = format!(
            "/{}/{}/{}{}",
            self.index, self.doc_type, self.id, self.options
        );
        let response = self.client.delete_op(&url)?;
        match response.status() {
            StatusCode::OK => Ok(response.read_response()?),
            _ => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                response.status()
            ))),
        }
    }
}

impl Client {
    /// Delete by ID
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/docs-delete.html
    pub fn delete<'a>(
        &'a mut self,
        index: &'a str,
        doc_type: &'a str,
        id: &'a str,
    ) -> DeleteOperation {
        DeleteOperation::new(self, index, doc_type, id)
    }
}

/// Result of a DELETE operation
#[derive(Debug, Deserialize)]
pub struct DeleteResult {
    pub found: bool,
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_type")]
    pub doc_type: String,
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_version")]
    pub version: u64,
}

#[cfg(test)]
pub mod tests {
    use tests::{clean_db, make_client, TestDocument};

    #[test]
    fn test_delete() {
        let index_name = "test_delete";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        let id = {
            let doc = TestDocument::new().with_int_field(4);
            let result = client
                .index(index_name, "test_type")
                .with_doc(&doc)
                .send()
                .unwrap();
            result.id
        };

        let delete_result = client.delete(index_name, "test_type", &id).send().unwrap();
        assert_eq!(id, delete_result.id);
        assert_eq!(true, delete_result.found);
    }
}
