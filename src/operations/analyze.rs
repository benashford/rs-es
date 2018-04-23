/*
 * Copyright 2015-2017 Ben Ashford
 * Copyright 2015 Astro
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

//! Implementation of ElasticSearch Analyze operation

use ::do_req;
use ::{Client, EsResponse};
use ::error::EsError;

#[derive(Debug)]
pub struct AnalyzeOperation<'a, 'b> {
    /// The HTTP client that this operation will use
    client:   &'a mut Client,

    body:     &'b str,
    index:    Option<&'b str>,
    analyzer: Option<&'b str>
}

impl<'a, 'b> AnalyzeOperation<'a, 'b> {
    pub fn new(client: &'a mut Client, body: &'b str) -> AnalyzeOperation<'a, 'b> {
        AnalyzeOperation {
            client:   client,
            body:     body,
            index:    None,
            analyzer: None
        }
    }

    pub fn with_index(&mut self, index: &'b str) -> &mut Self {
        self.index = Some(index);
        self
    }

    pub fn with_analyzer(&mut self, analyzer: &'b str) -> &mut Self {
        self.analyzer = Some(analyzer);
        self
    }

    pub fn send(&mut self) -> Result<AnalyzeResult, EsError> {
        let mut url = match self.index {
            None => "/_analyze".to_owned(),
            Some(index) => format!("{}/_analyze", index)
        };
        match self.analyzer {
            None => (),
            Some(analyzer) => {
                url.push_str(&format!("?analyzer={}", analyzer))
            }
        }
        let client = &self.client;
        let full_url = client.full_url(&url);
        let req = client.http_client
            .post(&full_url)
            .body(self.body)
            .send()?;
        let response = do_req(req)?;
        Ok(response.read_response()?)
    }
}

impl Client {
    /// Analyze
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/current/indices-analyze.html
    pub fn analyze<'a>(&'a mut self,
                       body: &'a str) -> AnalyzeOperation {
        AnalyzeOperation::new(self, body)
    }
}

/// The result of an analyze operation
#[derive(Debug, Deserialize)]
pub struct AnalyzeResult {
    pub tokens: Vec<Token>
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub token: String,
    #[serde(rename="type")]
    pub token_type: String,
    pub position: u64,
    pub start_offset: u64,
    pub end_offset: u64
}
