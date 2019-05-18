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

//! Refresh an Index

use reqwest::StatusCode;

use serde::Deserialize;

use crate::{error::EsError, Client, EsResponse};

use super::{format_multi, ShardCountResult};

#[derive(Debug)]
pub struct RefreshOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes being refreshed
    indexes: &'b [&'b str],
}

impl<'a, 'b> RefreshOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> RefreshOperation {
        RefreshOperation {
            client,
            indexes: &[],
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn send(&mut self) -> Result<RefreshResult, EsError> {
        let url = format!("/{}/_refresh", format_multi(&self.indexes));
        let response = self.client.post_op(&url)?;
        match response.status_code() {
            StatusCode::OK => Ok(response.read_response()?),
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }
}

impl Client {
    /// Refresh
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/indices-refresh.html
    pub fn refresh(&mut self) -> RefreshOperation {
        RefreshOperation::new(self)
    }
}

/// Result of a refresh request
#[derive(Deserialize)]
pub struct RefreshResult {
    #[serde(rename = "_shards")]
    pub shards: ShardCountResult,
}
