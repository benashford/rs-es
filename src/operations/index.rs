/*
 * Copyright 2015 Ben Ashford
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

use rustc_serialize::Encodable;
use rustc_serialize::json::Json;

use ::Client;
use ::error::EsError;
use super::common::Options;
use super::format_query_string;

/// Values for the op_type option
pub enum OpType {
    Create
}

impl ToString for OpType {
    fn to_string(&self) -> String {
        "create".to_string()
    }
}

/// An indexing operation
pub struct IndexOperation<'a, 'b, E: Encodable + 'b> {
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

impl<'a, 'b, E: Encodable + 'b> IndexOperation<'a, 'b, E> {
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
        let (_, result) = try!(match self.id {
            Some(ref id) => {
                let url = format!("/{}/{}/{}{}",
                                  self.index,
                                  self.doc_type,
                                  id,
                                  format_query_string(&mut self.options));
                match self.document {
                    Some(ref doc) => self.client.put_body_op(&url, doc),
                    None          => self.client.put_op(&url)
                }
            },
            None    => {
                let url = format!("/{}/{}{}",
                                  self.index,
                                  self.doc_type,
                                  format_query_string(&mut self.options));
                match self.document {
                    Some(ref doc) => self.client.post_body_op(&url, doc),
                    None          => self.client.post_op(&url)
                }
            }
        });
        Ok(IndexResult::from(&result.unwrap()))
    }
}

/// The result of an index operation
#[derive(Debug)]
pub struct IndexResult {
    pub index:    String,
    pub doc_type: String,
    pub id:       String,
    pub version:  i64,
    pub created:  bool
}

impl<'a> From<&'a Json> for IndexResult {
    fn from(r: &'a Json) -> IndexResult {
        IndexResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            version:  get_json_i64!(r, "_version"),
            created:  get_json_bool!(r, "created")
        }
    }
}
