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

use hyper::status::StatusCode;

use rustc_serialize::json::Json;

use ::{Client, EsResponse};
use ::error::EsError;
use super::common::{Options, OptionVal};

pub struct DeleteOperation<'a, 'b> {
    /// The HTTP client
    client:   &'a mut Client,

    /// The index
    index:    &'b str,

    /// The type
    doc_type: &'b str,

    /// The ID
    id:       &'b str,

    /// Optional options
    options:  Options<'b>
}

impl<'a, 'b> DeleteOperation<'a, 'b> {
    pub fn new(client:   &'a mut Client,
               index:    &'b str,
               doc_type: &'b str,
               id:       &'b str) -> DeleteOperation<'a, 'b> {
        DeleteOperation {
            client:   client,
            index:    index,
            doc_type: doc_type,
            id:       id,
            options:  Options::new()
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
        let url = format!("/{}/{}/{}{}",
                          self.index,
                          self.doc_type,
                          self.id,
                          self.options);
        let response = try!(self.client.delete_op(&url));
        match response.status_code() {
            &StatusCode::Ok =>
                Ok(try!(response.read_response())),
            _ =>
                Err(EsError::EsError(format!("Unexpected status: {}",
                                             response.status_code())))
        }
    }
}

/// Result of a DELETE operation
#[derive(Debug, Deserialize)]
pub struct DeleteResult {
    pub found:    bool,
    #[serde(rename="_index")]
    pub index:    String,
    #[serde(rename="_type")]
    pub doc_type: String,
    #[serde(rename="_id")]
    pub id:       String,
    #[serde(rename="_version")]
    pub version:  u64
}
