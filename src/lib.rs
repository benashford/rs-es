/*
 * Copyright 2015-2019 Ben Ashford
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

//! A client for ElasticSearch's REST API
//!
//! The `Client` itself is used as the central access point, from which numerous
//! operations are defined implementing each of the specific ElasticSearch APIs.
//!
//! Warning: at the time of writing the majority of such APIs are currently
//! unimplemented.

#[cfg(test)]
#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

#[macro_use]
pub mod util;

#[macro_use]
pub mod json;

pub mod error;
pub mod operations;
pub mod query;
pub mod units;

use std::time;

use reqwest::{header::CONTENT_TYPE, RequestBuilder, StatusCode, Url};

use serde::{de::DeserializeOwned, ser::Serialize};

use crate::error::EsError;

pub trait EsResponse {
    fn status_code(&self) -> StatusCode;
    fn read_response<R>(self) -> Result<R, EsError>
    where
        R: DeserializeOwned;
}

impl EsResponse for reqwest::Response {
    fn status_code(&self) -> StatusCode {
        self.status()
    }

    fn read_response<R>(self) -> Result<R, EsError>
    where
        R: DeserializeOwned,
    {
        Ok(serde_json::from_reader(self)?)
    }
}

// The client

/// Process the result of an HTTP request, returning the status code and the
/// `Json` result (if the result had a body) or an `EsError` if there were any
/// errors
///
/// This function is exposed to allow extensions to certain operations, it is
/// not expected to be used by consumers of the library
fn do_req(resp: reqwest::Response) -> Result<reqwest::Response, EsError> {
    let mut resp = resp;
    let status = resp.status();
    match status {
        StatusCode::OK | StatusCode::CREATED | StatusCode::NOT_FOUND => Ok(resp),
        _ => Err(EsError::from(&mut resp)),
    }
}

/// The core of the ElasticSearch client, owns a HTTP connection.
///
/// Each instance of `Client` is reusable, but only one thread can use each one
/// at once.  This will be enforced by the borrow-checker as most methods are
/// defined on `&mut self`.
///
/// To create a `Client`, the URL needs to be specified.
///
/// Each ElasticSearch API operation is defined as a method on `Client`.  Any
/// compulsory parameters must be given as arguments to this method.  It returns
/// an operation builder that can be used to add any optional parameters.
///
/// Finally `send` is called to submit the operation:
///
/// # Examples
///
/// ```
/// use rs_es::Client;
///
/// let mut client = Client::init("http://localhost:9200");
/// ```
///
/// See the specific operations and their builder objects for details.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: Url,
    http_client: reqwest::Client,
}

impl Client {
    fn do_es_op(
        &self,
        url: &str,
        action: impl FnOnce(Url) -> RequestBuilder,
    ) -> Result<reqwest::Response, EsError> {
        let url = self.full_url(url);
        let username = self.base_url.username();
        let mut method = action(url);
        if !username.is_empty() {
            method = method.basic_auth(username, self.base_url.password());
        }
        let result = method.header(CONTENT_TYPE, "application/json").send()?;
        do_req(result)
    }
}

/// Create a HTTP function for the given method (GET/PUT/POST/DELETE)
macro_rules! es_op {
    ($n:ident,$cn:ident) => {
        fn $n(&self, url: &str) -> Result<reqwest::Response, EsError> {
            log::info!("Doing {} on {}", stringify!($n), url);
            self.do_es_op(url, |url| self.http_client.$cn(url.clone()))
        }
    }
}

/// Create a HTTP function with a request body for the given method
/// (GET/PUT/POST/DELETE)
///
macro_rules! es_body_op {
    ($n:ident,$cn:ident) => {
        fn $n<E>(&mut self, url: &str, body: &E) -> Result<reqwest::Response, EsError>
            where E: Serialize {

            log::info!("Doing {} on {}", stringify!($n), url);
            let json_string = serde_json::to_string(body)?;
            log::debug!("With body: {}", &json_string);

            self.do_es_op(url, |url| {
                self.http_client.$cn(url.clone()).body(json_string)
            })
        }
    }
}

impl Client {
    /// Create a new client
    pub fn init(url_s: &str) -> Result<Client, reqwest::UrlError> {
        let url = Url::parse(url_s)?;

        Ok(Client {
            http_client: reqwest::Client::new(),
            base_url: url,
        })
    }

    // TODO - this should be replaced with a builder object, especially if more options are going
    // to be allowed
    pub fn init_with_timeout(
        url_s: &str,
        timeout: Option<time::Duration>,
    ) -> Result<Client, reqwest::UrlError> {
        let url = Url::parse(url_s)?;

        Ok(Client {
            http_client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to build client"),
            base_url: url,
        })
    }

    /// Take a nearly complete ElasticSearch URL, and stick
    /// the URL on the front.
    pub fn full_url(&self, suffix: &str) -> Url {
        self.base_url.join(suffix).expect("Invalid URL created")
    }

    es_op!(get_op, get);

    es_op!(post_op, post);
    es_body_op!(post_body_op, post);
    es_op!(put_op, put);
    es_body_op!(put_body_op, put);
    es_op!(delete_op, delete);
}

#[cfg(test)]
pub mod tests {
    use std::env;

    use serde::{Deserialize, Serialize};

    use super::{error::EsError, Client};

    // test setup

    pub fn make_client() -> Client {
        let hostname = match env::var("ES_HOST") {
            Ok(val) => val,
            Err(_) => "http://localhost:9200".to_owned(),
        };
        Client::init(&hostname).unwrap()
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TestDocument {
        pub str_field: String,
        pub int_field: i64,
        pub bool_field: bool,
    }

    #[allow(clippy::new_without_default)]
    impl TestDocument {
        pub fn new() -> TestDocument {
            TestDocument {
                str_field: "I am a test".to_owned(),
                int_field: 1,
                bool_field: true,
            }
        }

        pub fn with_str_field(mut self, s: &str) -> TestDocument {
            self.str_field = s.to_owned();
            self
        }

        pub fn with_int_field(mut self, i: i64) -> TestDocument {
            self.int_field = i;
            self
        }

        pub fn with_bool_field(mut self, b: bool) -> TestDocument {
            self.bool_field = b;
            self
        }
    }

    pub fn setup_test_data(client: &mut Client, index_name: &str) {
        // TODO - this should use the Bulk API
        let documents = vec![
            TestDocument::new()
                .with_str_field("Document A123")
                .with_int_field(1),
            TestDocument::new()
                .with_str_field("Document B456")
                .with_int_field(2),
            TestDocument::new()
                .with_str_field("Document 1ABC")
                .with_int_field(3),
        ];
        for doc in documents.iter() {
            client
                .index(index_name, "test_type")
                .with_doc(doc)
                .send()
                .unwrap();
        }
        client.refresh().with_indexes(&[index_name]).send().unwrap();
    }

    pub fn clean_db(client: &mut Client, test_idx: &str) {
        match client.delete_index(test_idx) {
            // Ignore indices which don't exist yet
            Err(EsError::EsError(ref msg)) if msg == "Unexpected status: 404 Not Found" => {}
            Ok(_) => {}
            e => {
                e.unwrap_or_else(|_| panic!("Failed to clean db for index {:?}", test_idx));
            }
        };
    }
}
