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

//! Fetch ElasticSearch version information

use ::Client;
use ::error::EsError;

pub struct VersionOperation<'a> {
    client: &'a mut Client
}

impl<'a> VersionOperation<'a> {
    pub fn new(client: &'a mut Client) -> Self {
        VersionOperation {
            client: client
        }
    }

    pub fn send(&mut self) -> Result<VersionResult, EsError> {
        let (_, result) = try!(self.client.get_op("/"));
        Ok(result)
    }
}

#[derive(Deserialize)]
pub struct Version {
    pub number: String,
    pub build_hash: String,
    pub build_timestamp: String,
    pub build_snapshot: bool,
    pub lucene_version: String
}

#[derive(Deserialize)]
pub struct VersionResult {
    pub name: String,
    pub cluster_name: String,
    pub version: Version,
    pub tagline: String,
}
