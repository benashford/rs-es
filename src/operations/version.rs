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

use ::{Client, EsResponse};
use ::error::EsError;

#[derive(Debug)]
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
        let response = self.client.get_op("/")?;
        Ok(response.read_response()?)
    }
}

impl Client {
    /// Calls the base ES path, returning the version number
    pub fn version(&mut self) -> VersionOperation {
        VersionOperation::new(self)
    }
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub number: String,
    pub build_hash: String,
    pub build_timestamp: String,
    pub build_snapshot: bool,
    pub lucene_version: String
}

#[derive(Debug, Deserialize)]
pub struct VersionResult {
    pub name: String,
    pub cluster_name: String,
    pub version: Version,
    pub tagline: String,
}

#[cfg(test)]
pub mod tests {
    use ::tests::{make_client};
    use ::tests::regex::Regex;

    #[test]
    fn it_works() {
        let mut client = make_client();
        let result = client.version().send().unwrap();

        let expected_regex = Regex::new(r"^\d\.\d\.\d$").unwrap();
        assert_eq!(expected_regex.is_match(&result.version.number), true);
    }
}
