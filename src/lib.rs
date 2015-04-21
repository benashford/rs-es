#![crate_type = "lib"]
#![crate_name = "rs_es"]

#![feature(convert)]
#![feature(std_misc)]

#[macro_use] extern crate log;
extern crate hyper;
extern crate rustc_serialize;

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::fmt;

use rustc_serialize::json;
use rustc_serialize::json::{Json, ToJson};

// Error handling

#[derive(Debug)]
pub enum EsError {
    EsError(String),
    HttpError(hyper::error::HttpError),
    IoError(io::Error),
    JsonBuilderError(json::BuilderError)
}

impl From<io::Error> for EsError {
    fn from(err: io::Error) -> EsError {
        EsError::IoError(err)
    }
}

impl From<hyper::error::HttpError> for EsError {
    fn from(err: hyper::error::HttpError) -> EsError {
        EsError::HttpError(err)
    }
}

impl From<json::BuilderError> for EsError {
    fn from(err: json::BuilderError) -> EsError {
        EsError::JsonBuilderError(err)
    }
}

impl Error for EsError {
    fn description(&self) -> &str {
        match *self {
            EsError::EsError(ref err) => err.as_str(),
            EsError::HttpError(ref err) => err.description(),
            EsError::IoError(ref err) => err.description(),
            EsError::JsonBuilderError(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EsError::EsError(_)         => None,
            EsError::HttpError(ref err) => Some(err as &Error),
            EsError::IoError(ref err)   => Some(err as &Error),
            EsError::JsonBuilderError(ref err) => Some(err as &Error)
        }
    }
}

impl fmt::Display for EsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EsError::EsError(ref s) => fmt::Display::fmt(s, f),
            EsError::HttpError(ref err) => fmt::Display::fmt(err, f),
            EsError::IoError(ref err) => fmt::Display::fmt(err, f),
            EsError::JsonBuilderError(ref err) => fmt::Display::fmt(err, f)
        }
    }
}

// Utilities

fn format_query_string(options: &mut HashMap<&'static str, String>) -> String {
    let mut st = String::new();
    if options.is_empty() {
        return st;
    }
    st.push_str("?");
    for (k, v) in options.drain() {
        st.push_str(k);
        st.push_str("=");
        st.push_str(v.as_str());
        st.push_str("&");
    }
    st.pop();
    st
}

// The client

fn do_req<'a>(rb:   hyper::client::RequestBuilder<'a, &str>,
              body: Option<&'a str>) -> Result<Json, EsError> {
    info!("Params (body={:?})", body.unwrap());
    let mut result = match body {
        Some(json_str) => rb.body(json_str).send(),
        None           => rb.send()
    };
    info!("Result: {:?}", result);
    match result {
        Ok(ref mut r) => match Json::from_reader(r) {
            Ok(json) => Ok(json),
            Err(e)   => Err(EsError::from(e))
        },
        Err(e)        => Err(EsError::from(e))
    }
}

pub struct Client {
    host:        String,
    port:        u32,
    http_client: hyper::Client
}

macro_rules! es_op {
    ($n:ident) => {
        fn $n(&mut self, url: &str, body: Option<&Json>) -> Result<Json, EsError> {
            info!("Doing $n on {} with {:?}", url, body);
            match body {
                Some(json) => {
                    let json_string = json::encode(json).unwrap();
                    do_req(self.http_client.$n(url), Some(json_string.as_str()))
                },
                None => {
                    do_req(self.http_client.$n(url), None)
                }
            }
        }
    }
}

impl Client {
    pub fn new(host: String, port: u32) -> Client {
        Client {
            host:        host,
            port:        port,
            http_client: hyper::Client::new()
        }
    }

    fn get_base_url(&self) -> String {
        format!("http://{}:{}/", self.host, self.port)
    }

    es_op!(get);
    es_op!(post);
    es_op!(put);

    pub fn version(&mut self) -> Result<String, EsError> {
        let url = self.get_base_url();
        let json = try!(self.get(url.as_str(), None));
        match json.find_path(&["version", "number"]) {
            Some(version) => match version.as_string() {
                Some(string) => Ok(string.to_string()),
                None         => Err(EsError::EsError(format!("Cannot find version number in: {:?}",
                                                             json)))
            },
            None          => Err(EsError::EsError(format!("Cannot find version number in {:?}",
                                                          json)))
        }
    }

    pub fn index<'a>(&'a mut self, index: &'a str, doc_type: &'a str) -> IndexOperation {
        IndexOperation::new(self, index, doc_type)
    }
}

// Specific operations

type Options = HashMap<&'static str, String>;

macro_rules! add_option {
    ($n:ident, $e:expr, $t:ident) => (
        pub fn $n<T: ToString>(&'a mut self, val: &T) -> &'a mut $t {
            self.options.insert($e, val.to_string());
            self
        }
    )
}

pub struct IndexOperation<'a> {
    client:   &'a mut Client,
    index:    &'a str,
    doc_type: &'a str,
    id:       Option<&'a str>,
    options:  Options,
    document: Option<Json>
}

impl<'a> IndexOperation<'a> {
    fn new(client: &'a mut Client, index: &'a str, doc_type: &'a str) -> IndexOperation<'a> {
        IndexOperation {
            client:   client,
            index:    index,
            doc_type: doc_type,
            id:       None,
            options:  HashMap::<&'static str, String>::new(),
            document: None
        }
    }

    pub fn with_doc<T: ToJson>(&'a mut self, doc: &T) -> &'a mut IndexOperation {
        self.document = Some(doc.to_json());
        self
    }

    pub fn with_id(&'a mut self, id: &'a str) -> &'a mut IndexOperation {
        self.id = Some(id);
        self
    }

    add_option!(with_ttl, "ttl", IndexOperation);

    pub fn send(&'a mut self) -> Result<Json, EsError> {
        match self.id {
            Some(id) => {
                let url = format!("{}{}/{}/{}{}",
                                  self.client.get_base_url(),
                                  self.index,
                                  self.doc_type,
                                  id,
                                  format_query_string(&mut self.options));
                self.client.put(url.as_str(), match self.document {
                    Some(ref doc) => Some(doc),
                    None          => None
                })
            },
            None    => {
                let url = format!("{}{}/{}{}",
                                  self.client.get_base_url(),
                                  self.index,
                                  self.doc_type,
                                  format_query_string(&mut self.options));
                self.client.post(url.as_str(), match self.document {
                    Some(ref doc) => Some(doc),
                    None          => None
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;

    use super::Client;

    use std::collections::BTreeMap;

    use rustc_serialize::json;
    use rustc_serialize::json::{Json, ToJson};

    // test setup

    fn make_client() -> Client {
        Client::new("localhost".to_string(), 9200)
    }

    struct TestDocument {
        str_field: String
    }

    impl ToJson for TestDocument {
        fn to_json(&self) -> Json {
            let mut d = BTreeMap::new();
            d.insert("str_field".to_string(), self.str_field.to_json());

            Json::Object(d)
        }
    }

    fn make_doc() -> TestDocument {
        TestDocument {
            str_field: "I am a test".to_string()
        }
    }

    // tests

    #[test]
    fn it_works() {
        let mut client = make_client();
        assert_eq!(client.version().unwrap(), "1.3.2");
    }

    #[test]
    fn test_indexing() {
        env_logger::init().unwrap();

        let mut client = make_client();
        let mut indexer = client.index("test_idx", "test_type");
        let doc = make_doc();
        let result = indexer.with_doc(&doc).with_ttl(&927500).send();
        info!("TEST RESULT: {:?}", result);
        assert_eq!(json::encode(&result.unwrap()).unwrap(), "");
    }
}
