#![crate_type = "lib"]
#![crate_name = "rs_es"]

#![feature(collections)]
#![feature(convert)]

#[macro_use] extern crate log;
extern crate hyper;
extern crate rustc_serialize;

use std::error::Error;
use std::io;
use std::fmt;

use hyper::client::response;
use hyper::status::StatusCode;

use rustc_serialize::json;
use rustc_serialize::json::{Json, ToJson};

// Error handling

#[derive(Debug)]
pub enum EsError {
    EsError(String),
    EsServerError(String),
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

impl<'a> From<&'a mut response::Response> for EsError {
    fn from(err: &'a mut response::Response) -> EsError {
        EsError::EsServerError(format!("{} - {:?}", err.status, err))
    }
}

impl Error for EsError {
    fn description(&self) -> &str {
        match *self {
            EsError::EsError(ref err) => err.as_str(),
            EsError::EsServerError(ref err) => err.as_str(),
            EsError::HttpError(ref err) => err.description(),
            EsError::IoError(ref err) => err.description(),
            EsError::JsonBuilderError(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EsError::EsError(_)                => None,
            EsError::EsServerError(_)          => None,
            EsError::HttpError(ref err)        => Some(err as &Error),
            EsError::IoError(ref err)          => Some(err as &Error),
            EsError::JsonBuilderError(ref err) => Some(err as &Error)
        }
    }
}

impl fmt::Display for EsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EsError::EsError(ref s) => fmt::Display::fmt(s, f),
            EsError::EsServerError(ref s) => fmt::Display::fmt(s, f),
            EsError::HttpError(ref err) => fmt::Display::fmt(err, f),
            EsError::IoError(ref err) => fmt::Display::fmt(err, f),
            EsError::JsonBuilderError(ref err) => fmt::Display::fmt(err, f)
        }
    }
}

// Utilities

fn format_query_string(options: &mut Vec<(&'static str, String)>) -> String {
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
              body: Option<&'a str>) -> Result<Option<Json>, EsError> {
    info!("Params (body={:?})", body.unwrap());
    let mut result = match body {
        Some(json_str) => rb.body(json_str).send(),
        None           => rb.send()
    };
    info!("Result: {:?}", result);
    match result {
        Ok(ref mut r) => match r.status {
            StatusCode::Ok |
            StatusCode::Created  => match Json::from_reader(r) {
                Ok(json) => Ok(Some(json)),
                Err(e)   => Err(EsError::from(e))
            },
            StatusCode::NotFound => Ok(None),
            _                    => Err(EsError::from(r))
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
        fn $n(&mut self, url: &str, body: Option<&Json>) -> Result<Option<Json>, EsError> {
            info!("Doing {} on {} with {:?}", stringify!($n), url, body);
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
        let json = try!(self.get(url.as_str(), None)).unwrap();
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

type Options = Vec<(&'static str, String)>;

macro_rules! add_option {
    ($n:ident, $e:expr, $t:ident) => (
        pub fn $n<T: ToString>(&'a mut self, val: &T) -> &'a mut $t {
            self.options.push(($e, val.to_string()));
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
            options:  Options::new(),
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
    add_option!(with_version, "version", IndexOperation);
    add_option!(with_version_type, "version_type", IndexOperation);
    add_option!(with_op_type, "op_type", IndexOperation);
    add_option!(with_routing, "routing", IndexOperation);
    add_option!(with_parent, "parent", IndexOperation);
    add_option!(with_timestamp, "timestamp", IndexOperation);
    add_option!(with_refresh, "refresh", IndexOperation);
    add_option!(with_timeout, "timeout", IndexOperation);

    pub fn send(&'a mut self) -> Result<IndexResult, EsError> {
        let result = try!(match self.id {
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
        });
        Ok(IndexResult::from(result.unwrap()))
    }
}

macro_rules! get_json_thing {
    ($r:ident,$f:expr,$t:ident) => {
        $r.find($f).unwrap().$t().unwrap()
    }
}

macro_rules! get_json_string {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_string).to_string()
    }
}

macro_rules! get_json_i64 {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_i64)
    }
}

macro_rules! get_json_bool {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_boolean)
    }
}

#[derive(Debug)]
pub struct IndexResult {
    index:    String,
    doc_type: String,
    id:       String,
    version:  i64,
    created:  bool
}

impl From<Json> for IndexResult {
    fn from(r: Json) -> IndexResult {
        IndexResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            version:  get_json_i64!(r, "_version"),
            created:  get_json_bool!(r, "created")
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    extern crate regex;

    use super::Client;

    use std::collections::BTreeMap;

    use self::regex::Regex;

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
        let result = client.version().unwrap();

        let expected_regex = Regex::new(r"^\d\.\d\.\d$").unwrap();
        assert_eq!(expected_regex.is_match(result.as_str()), true);
    }

    #[test]
    fn test_indexing() {
        env_logger::init().unwrap();

        let mut client = make_client();
        let mut indexer = client.index("test_idx", "test_type");
        let doc = make_doc();
        let result_wrapped = indexer.with_doc(&doc).with_ttl(&927500).send();
        info!("TEST RESULT: {:?}", result_wrapped);
        let result = result_wrapped.unwrap();
        assert_eq!(result.created, true);
        assert_eq!(result.index, "test_idx");
        assert_eq!(result.doc_type, "test_type");
        assert!(result.id.len() > 0);
        assert_eq!(result.version, 1);
    }
}
