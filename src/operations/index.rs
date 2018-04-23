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

//! Implementation of ElasticSearch Index operation

use serde::ser::Serialize;

use ::{Client, EsResponse};
use ::error::EsError;
use super::common::{Options, OptionVal};

/// Values for the op_type option
pub enum OpType {
    Create
}

impl From<OpType> for OptionVal {
    fn from(from: OpType) -> OptionVal {
        match from {
            OpType::Create => OptionVal("create".to_owned())
        }
    }
}

/// An indexing operation
#[derive(Debug)]
pub struct IndexOperation<'a, 'b, E: Serialize + 'b> {
    /// The HTTP client that this operation will use
    client:   &'a mut Client,

    /// The index into which the document will be added
    index:    &'b str,

    /// The type of the document
    doc_type: &'b str,

    /// Optional the ID of the document.
    id:       Option<&'b str>,

    /// The optional options
    options:  Options<'b>,

    /// The document to be indexed
    document: Option<&'b E>
}

impl<'a, 'b, E: Serialize + 'b> IndexOperation<'a, 'b, E> {
    pub fn new(client: &'a mut Client, index: &'b str, doc_type: &'b str) -> IndexOperation<'a, 'b, E> {
        IndexOperation {
            client:   client,
            index:    index,
            doc_type: doc_type,
            id:       None,
            options:  Options::new(),
            document: None
        }
    }

    pub fn with_doc(&'b mut self, doc: &'b E) -> &'b mut Self {
        self.document = Some(doc);
        self
    }

    pub fn with_id(&'b mut self, id: &'b str) -> &'b mut Self {
        self.id = Some(id);
        self
    }

    add_option!(with_ttl, "ttl");
    add_option!(with_version, "version");
    add_option!(with_version_type, "version_type");
    add_option!(with_op_type, "op_type");
    add_option!(with_routing, "routing");
    add_option!(with_parent, "parent");
    add_option!(with_timestamp, "timestamp");
    add_option!(with_refresh, "refresh");
    add_option!(with_timeout, "timeout");

    pub fn send(&'b mut self) -> Result<IndexResult, EsError> {
        // Ignoring status_code as everything should return an IndexResult or
        // already be an error
        let response = (match self.id {
            Some(ref id) => {
                let url = format!("/{}/{}/{}{}",
                                  self.index,
                                  self.doc_type,
                                  id,
                                  self.options);
                match self.document {
                    Some(ref doc) => self.client.put_body_op(&url, doc),
                    None          => self.client.put_op(&url)
                }
            },
            None    => {
                let url = format!("/{}/{}{}",
                                  self.index,
                                  self.doc_type,
                                  self.options);
                match self.document {
                    Some(ref doc) => self.client.post_body_op(&url, doc),
                    None          => self.client.post_op(&url)
                }
            }
        })?;
        Ok(response.read_response()?)
    }
}

impl Client {
    /// An index operation to index a document in the specified index.
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/docs-index_.html
    pub fn index<'a, 'b, E: Serialize>(&'a mut self, index: &'b str, doc_type: &'b str)
                                       -> IndexOperation<'a, 'b, E> {
        IndexOperation::new(self, index, doc_type)
    }
}

/// The result of an index operation
#[derive(Debug, Deserialize)]
pub struct IndexResult {
    #[serde(rename="_index")]
    pub index:    String,
    #[serde(rename="_type")]
    pub doc_type: String,
    #[serde(rename="_id")]
    pub id:       String,
    #[serde(rename="_version")]
    pub version:  u64,
    pub created:  bool
}

#[cfg(test)]
pub mod tests {
    use ::tests::{clean_db, TestDocument, make_client};

    use ::units::Duration;

    use super::OpType;

    #[test]
    fn test_indexing() {
        let index_name = "test_indexing";
        let mut client = make_client();
        clean_db(&mut client, index_name);
        {
            let result_wrapped = client
                .index(index_name, "test_type")
                .with_doc(&TestDocument::new().with_int_field(1))
                .with_ttl(&Duration::milliseconds(927500))
                .send();
            println!("TEST RESULT: {:?}", result_wrapped);
            let result = result_wrapped.unwrap();
            assert_eq!(result.created, true);
            assert_eq!(result.index, index_name);
            assert_eq!(result.doc_type, "test_type");
            assert!(result.id.len() > 0);
            assert_eq!(result.version, 1);
        }
        {
            let result_wrapped = client
                .index(index_name, "test_type")
                .with_doc(&TestDocument::new().with_int_field(2))
                .with_id("TEST_INDEXING_2")
                .with_op_type(OpType::Create)
                .send();
            let result = result_wrapped.unwrap();

            assert_eq!(result.created, true);
            assert_eq!(result.index, index_name);
            assert_eq!(result.doc_type, "test_type");
            assert_eq!(result.id, "TEST_INDEXING_2");
            assert!(result.version >= 1);
        }
    }
}
