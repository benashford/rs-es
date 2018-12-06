/*
 * Copyright 2015-2018 Ben Ashford
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

#![crate_type = "lib"]
#![crate_name = "rs_es"]

//! A client for ElasticSearch's REST API
//!
//! The `Client` itself is used as the central access point, from which numerous
//! operations are defined implementing each of the specific ElasticSearch APIs.
//!
//! Warning: at the time of writing the majority of such APIs are currently
//! unimplemented.

#[macro_use]
extern crate serde_derive;

extern crate serde;

#[cfg_attr(test, macro_use)]
extern crate serde_json;

#[macro_use]
extern crate log;
extern crate reqwest;

#[macro_use]
extern crate maplit;

#[macro_use]
pub mod util;

#[macro_use]
pub mod json;

pub mod error;
pub mod operations;
pub mod query;
pub mod units;

use reqwest::{StatusCode, Url};

use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use error::EsError;

pub trait EsResponse {
    fn read_response<R>(self) -> Result<R, EsError>
    where
        R: DeserializeOwned;
}

impl EsResponse for reqwest::Response {
    fn read_response<R>(self) -> Result<R, EsError>
    where
        R: DeserializeOwned,
    {
        serde_json::from_reader(self).map_err(|err| EsError::from(err))
    }
}

// The client

/// Process the result of an HTTP request, returning the status code and the
/// `Json` result (if the result had a body) or an `EsError` if there were any
/// errors
///
/// This function is exposed to allow extensions to certain operations, it is
/// not expected to be used by consumers of the library
pub fn do_req(resp: reqwest::Response) -> Result<reqwest::Response, EsError> {
    let mut resp = resp;
    let status = resp.status();
    match status {
        StatusCode::OK | StatusCode::CREATED | StatusCode::NOT_FOUND => Ok(resp),
        _ => Err(EsError::from(&mut resp)),
    }
}

/// The client builder, used to wrap up a reqwest Client for use in ElasticSearch.
#[derive(Debug)]
pub struct ClientBuilder {
    base_url: String,
    client: Option<reqwest::Client>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder {
            base_url: "http://localhost:9200".into(),
            client: None,
        }
    }

    pub fn with_base_url<I>(mut self, url: I) -> Self
    where
        I: Into<String>,
    {
        self.base_url = url.into();
        self
    }

    /* XXX: TODO: Basic auth
    pub fn basic_auth(url: &Url) -> Headers {
        let mut headers = Headers::new();

        let username = url.username();

        if !username.is_empty() {
            headers.set(Authorization(Basic {
                username: username.to_owned(),
                password: url.password().map(|p| p.to_owned()),
            }))
        }

        headers.set(ContentType::json());

        headers
    }
    */

    pub fn with_client<I>(mut self, client: reqwest::Client) -> Self
    where
        I: Into<String>,
    {
        self.client = Some(client);
        self
    }

    pub fn build(self) -> Result<Client, EsError> {
        let client = self.client.unwrap_or_else(|| reqwest::Client::new());
        let base_url = Url::parse(&self.base_url)?;
        let username = match base_url.username() {
            "" => None,
            _ => Some(base_url.username().to_string()),
        };
        let password = match base_url.password() {
            Some(password) => Some(password.to_string()),
            None => None,
        };

        Ok(Client {
            base_url,
            client,
            username,
            password,
        })
    }
}

/// The core of the ElasticSearch client, owns a HTTP connection.
///
/// XXX: THIS IS OUT OF DATE
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
/// let mut client = Client::new("http://localhost:9200");
/// ```
///
/// See the specific operations and their builder objects for details.
#[derive(Debug)]
pub struct Client {
    base_url: Url,
    client: reqwest::Client,
    username: Option<String>,
    password: Option<String>,
}

/// Create a HTTP function for the given method (GET/PUT/POST/DELETE)
macro_rules! es_op {
    ($n:ident,$cn:ident) => {
        fn $n(&mut self, url: &str) -> Result<reqwest::Response, EsError> {
            info!("Doing {} on {}", stringify!($n), url);
            let url = self.full_url(url);
            let mut result = self.client.$cn(&url);
            if let Some(username) = &self.username {
                result = result.basic_auth(username.clone(), self.password.clone());
            }
            do_req(result.send()?)
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

            info!("Doing {} on {}", stringify!($n), url);
            let json_string = serde_json::to_string(body)?;
            debug!("Body send: {}", &json_string);

            let url = self.full_url(url);
            let mut result = self.client
                .$cn(&url)
                .body(json_string);
            if let Some(username) = &self.username {
                result = result.basic_auth(username.clone(), self.password.clone());
            }
            do_req(result.send()?)
        }
    }
}

impl Client {
    /// Take a nearly complete ElasticSearch URL, and stick
    /// the URL on the front.
    pub fn full_url(&self, suffix: &str) -> String {
        self.base_url.join(suffix).unwrap().into_string()
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
    extern crate env_logger;
    pub extern crate regex;

    use std::env;

    use serde_json::Value;

    use super::operations::bulk::Action;
    use super::operations::search::ScanResult;
    use super::{Client, ClientBuilder};

    use super::query::Query;

    use super::units::Duration;

    // test setup

    pub fn make_client() -> Client {
        let hostname = match env::var("ES_HOST") {
            Ok(val) => val,
            Err(_) => "http://localhost:9200".to_owned(),
        };
        ClientBuilder::new()
            .with_base_url(hostname)
            .build()
            .unwrap()
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TestDocument {
        pub str_field: String,
        pub int_field: i64,
        pub bool_field: bool,
    }

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
        for ref doc in documents {
            client
                .index(index_name, "test_type")
                .with_doc(doc)
                .send()
                .unwrap();
        }
        client.refresh().with_indexes(&[index_name]).send().unwrap();
    }

    pub fn clean_db(mut client: &mut Client, test_idx: &str) {
        // let's do some logging
        let _ = env_logger::init();

        let scroll = Duration::minutes(1);
        let mut scan: ScanResult<Value> = match client
            .search_query()
            .with_indexes(&[test_idx])
            .with_query(&Query::build_match_all().build())
            .scan(&scroll)
        {
            Ok(scan) => scan,
            Err(e) => {
                warn!("Scan error: {:?}", e);
                return; // Ignore not-found errors
            }
        };

        loop {
            let page = scan.scroll(&mut client, &scroll).unwrap();
            let hits = page.hits.hits;
            if hits.is_empty() {
                break;
            }
            let actions: Vec<Action<()>> = hits
                .into_iter()
                .map(|hit| {
                    Action::delete(hit.id)
                        .with_index(test_idx)
                        .with_doc_type(hit.doc_type)
                }).collect();
            client.bulk(&actions).send().unwrap();
        }

        scan.close(&mut client).unwrap();
    }
}
