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

//! Implementations of specific ElasticSearch operations
//!
//! The various methods on [`Client`](../struct.Client.html) are entry points to
//! ElasticSearch's set of operations.  This module, and it's child modules are
//! the implementation of those operations.

use hyper::status::StatusCode;

use ::{Client, EsResponse};
use ::error::EsError;
use ::util::StrJoin;

// Specific operations
#[macro_use]
pub mod common;

pub mod bulk;
pub mod delete;
pub mod get;
pub mod index;
pub mod search;
pub mod analyze;
pub mod mapping;
pub mod version;

// Common utility functions

/// A repeating convention in the ElasticSearch REST API is parameters that can
/// take multiple values
fn format_multi(parts: &[&str]) -> String {
    if parts.is_empty() {
        return "_all".to_owned()
    } else {
        parts.iter().join(",")
    }
}

/// Multiple operations require indexes and types to be specified, there are
/// rules for combining the two however.  E.g. all indexes is specified with
/// `_all`, but all types are specified by omitting type entirely.
fn format_indexes_and_types(indexes: &[&str], types: &[&str]) -> String {
    if types.len() == 0 {
        format!("{}", format_multi(indexes))
    } else {
        format!("{}/{}", format_multi(indexes), format_multi(types))
    }
}

// TODO: move to refresh.rs
pub struct RefreshOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes being refreshed
    indexes: &'b [&'b str]
}

impl<'a, 'b> RefreshOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> RefreshOperation {
        RefreshOperation {
            client:  client,
            indexes: &[]
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn send(&mut self) -> Result<RefreshResult, EsError> {
        let url = format!("/{}/_refresh",
                          format_multi(&self.indexes));
        let response = try!(self.client.post_op(&url));
        match response.status_code() {
            &StatusCode::Ok => Ok(try!(response.read_response())),
            _              => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

// Results

/// Shared struct for operations that include counts of success/failed shards.
/// This is returned within various other result structs.
#[derive(Debug, Deserialize)]
pub struct ShardCountResult {
    pub total:      u64,
    pub successful: u64,
    pub failed:     u64
}

/// Result of a refresh request
#[derive(Deserialize)]
pub struct RefreshResult {
    #[serde(rename="_shards")]
    pub shards: ShardCountResult
}
