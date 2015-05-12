#![crate_type = "lib"]
#![crate_name = "rs_es"]

//! A client for ElasticSearch's REST API

#[macro_use] extern crate log;
extern crate hyper;
extern crate rustc_serialize;

pub mod query;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::fmt;

use hyper::client::response;
use hyper::status::StatusCode;

use rustc_serialize::{Encodable, Decodable};
use rustc_serialize::json::{self, Decoder, Json, ToJson};

use query::Query;

// Error handling

/// Error that can occur include IO and parsing errors, as well as specific
/// errors from the ElasticSearch server and logic errors from this library
#[derive(Debug)]
pub enum EsError {
    /// An internal error from this library
    EsError(String),

    /// An error reported in a JSON response from the ElasticSearch server
    EsServerError(String),

    /// Miscellaneous error from the HTTP library
    HttpError(hyper::error::Error),

    /// Miscellaneous IO error
    IoError(io::Error),

    /// Miscellaneous JSON decoding error
    JsonError(json::DecoderError),

    /// Miscllenaeous JSON building error
    JsonBuilderError(json::BuilderError)
}

impl From<io::Error> for EsError {
    fn from(err: io::Error) -> EsError {
        EsError::IoError(err)
    }
}

impl From<hyper::error::Error> for EsError {
    fn from(err: hyper::error::Error) -> EsError {
        EsError::HttpError(err)
    }
}

impl From<json::DecoderError> for EsError {
    fn from(err: json::DecoderError) -> EsError {
        EsError::JsonError(err)
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
            EsError::EsError(ref err) => err,
            EsError::EsServerError(ref err) => err,
            EsError::HttpError(ref err) => err.description(),
            EsError::IoError(ref err) => err.description(),
            EsError::JsonError(ref err) => err.description(),
            EsError::JsonBuilderError(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EsError::EsError(_)                => None,
            EsError::EsServerError(_)          => None,
            EsError::HttpError(ref err)        => Some(err as &Error),
            EsError::IoError(ref err)          => Some(err as &Error),
            EsError::JsonError(ref err)        => Some(err as &Error),
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
            EsError::JsonError(ref err) => fmt::Display::fmt(err, f),
            EsError::JsonBuilderError(ref err) => fmt::Display::fmt(err, f)
        }
    }
}

// Utilities

/// Produces a query string for a URL
fn format_query_string(options: &[(&str, String)]) -> String {
    let mut st = String::new();
    if options.is_empty() {
        return st;
    }
    st.push_str("?");
    for &(ref k, ref v) in options {
        st.push_str(k);
        st.push_str("=");
        st.push_str(&v);
        st.push_str("&");
    }
    st.pop();
    st
}

/// A repeating convention in the ElasticSearch REST API is parameters that can
/// take multiple values
fn format_multi(parts: &[&str]) -> String {
    let mut st = String::new();
    if parts.is_empty() {
        st.push_str("_all");
    } else {
        for s in parts {
            st.push_str(s);
            st.push_str(",");
        }
        st.pop();
    }
    st
}

// The client

/// Perform an HTTP request
fn do_req<'a>(rb:   hyper::client::RequestBuilder<'a, &str>,
              body: Option<&'a str>)
              -> Result<(StatusCode, Option<Json>), EsError> {
    info!("Params (body={:?})", body);
    let mut result = match body {
        Some(json_str) => rb.body(json_str).send(),
        None           => rb.send()
    };
    info!("Result: {:?}", result);
    match result {
        Ok(ref mut r) => match r.status {
            StatusCode::Ok |
            StatusCode::Created |
            StatusCode::NotFound => match Json::from_reader(r) {
                Ok(json) => Ok((r.status, Some(json))),
                Err(e)   => Err(EsError::from(e))
            },
            _                    => Err(EsError::from(r))
        },
        Err(e)        => Err(EsError::from(e))
    }
}

/// The core of the ElasticSearch client, owns a HTTP connection
pub struct Client {
    base_url:    String,
    http_client: hyper::Client
}

/// Create a HTTP function for the given method (GET/PUT/POST/DELETE)
macro_rules! es_op {
    ($n:ident,$cn:ident) => {
        fn $n(&mut self, url: &str)
              -> Result<(StatusCode, Option<Json>), EsError> {
            info!("Doing {} on {}", stringify!($n), url);
            do_req(self.http_client.$cn(&format!("{}/{}", self.base_url, url)), None)
        }
    }
}

/// Create a HTTP function with a request body for the given method
/// (GET/PUT/POST/DELETE)
macro_rules! es_body_op {
    ($n:ident,$cn:ident) => {
        fn $n<E>(&mut self, url: &str, body: &E)
                 -> Result<(StatusCode, Option<Json>), EsError>
            where E: Encodable {
                info!("Doing {} on {}", stringify!($n), url);

                let json_string = json::encode(body).unwrap();
                info!(" -> body: {:?}", json_string);
                do_req(self.http_client.$cn(&format!("{}/{}", self.base_url, url)),
                       Some(&json_string))
            }
    }
}

impl Client {
    /// Create a new client
    pub fn new(host: &str, port: u32) -> Client {
        Client {
            base_url:    format!("http://{}:{}", host, port),
            http_client: hyper::Client::new()
        }
    }

    es_op!(get_op, get);
    //TODO: enable this, required for ES Search API es_body_op!(get_body_op, get);
    es_op!(post_op, post);
    es_body_op!(post_body_op, post);
    es_op!(put_op, put);
    es_body_op!(put_body_op, put);
    es_op!(delete_op, delete);
    es_body_op!(delete_body_op, delete);

    /// Calls the base ES path, returning the version number
    pub fn version(&mut self) -> Result<String, EsError> {
        let (_, result) = try!(self.get_op("/"));
        let json = result.unwrap();
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

    /// An index operation to index a document in the specified index
    pub fn index<'a, 'b, E: Encodable>(&'a mut self, index: &'b str, doc_type: &'b str)
                                       -> IndexOperation<'a, 'b, E> {
        IndexOperation::new(self, index, doc_type)
    }

    /// Implementation of the ES GET API
    pub fn get<'a>(&'a mut self,
                   index: &'a str,
                   id:    &'a str) -> GetOperation {
        GetOperation::new(self, index, id)
    }

    /// Delete by ID
    pub fn delete<'a>(&'a mut self,
                      index:    &'a str,
                      doc_type: &'a str,
                      id:       &'a str) -> DeleteOperation {
        DeleteOperation::new(self, index, doc_type, id)
    }

    /// Delete by query
    pub fn delete_by_query<'a>(&'a mut self) -> DeleteByQueryOperation {
        DeleteByQueryOperation::new(self)
    }
}

// Specific operations

/// Every ES operation has a set of options
type Options<'a> = Vec<(&'a str, String)>;

/// Values for the op_type option
pub enum OpType {
    Create
}

impl ToString for OpType {
    fn to_string(&self) -> String {
        "create".to_string()
    }
}

/// Adds a function to an operation to add specific options to that operations
/// builder interface.
macro_rules! add_option {
    ($n:ident, $e:expr) => (
        pub fn $n<T: ToString>(&'a mut self, val: &T) -> &'a mut Self {
            self.options.push(($e, val.to_string()));
            self
        }
    )
}

/// An indexing operation
pub struct IndexOperation<'a, 'b, E: Encodable + 'b> {
    /// The HTTP client that this operation will use
    client:   &'a mut Client,

    /// The index into which the document will be added
    index:    &'b str,

    /// The type of the document
    doc_type: &'b str,

    /// Optional the ID of the document.
    id:       Option<&'b str>,

    /// The optional options
    options:  Options<'b>,

    /// The document to be indexed
    document: Option<&'b E>
}

impl<'a, 'b, E: Encodable + 'b> IndexOperation<'a, 'b, E> {
    fn new(client: &'a mut Client, index: &'b str, doc_type: &'b str) -> IndexOperation<'a, 'b, E> {
        IndexOperation {
            client:   client,
            index:    index,
            doc_type: doc_type,
            id:       None,
            options:  Options::new(),
            document: None
        }
    }

    pub fn with_doc(&'b mut self, doc: &'b E) -> &'b mut Self {
        self.document = Some(doc);
        self
    }

    pub fn with_id(&'b mut self, id: &'b str) -> &'b mut Self {
        self.id = Some(id);
        self
    }

    add_option!(with_ttl, "ttl");
    add_option!(with_version, "version");
    add_option!(with_version_type, "version_type");
    add_option!(with_op_type, "op_type");
    add_option!(with_routing, "routing");
    add_option!(with_parent, "parent");
    add_option!(with_timestamp, "timestamp");
    add_option!(with_refresh, "refresh");
    add_option!(with_timeout, "timeout");

    pub fn send(&'b mut self) -> Result<IndexResult, EsError> {
        // Ignoring status_code as everything should return an IndexResult or
        // already be an error
        let (_, result) = try!(match self.id {
            Some(ref id) => {
                let url = format!("/{}/{}/{}{}",
                                  self.index,
                                  self.doc_type,
                                  id,
                                  format_query_string(&mut self.options));
                match self.document {
                    Some(ref doc) => self.client.put_body_op(&url, doc),
                    None          => self.client.put_op(&url)
                }
            },
            None    => {
                let url = format!("/{}/{}{}",
                                  self.index,
                                  self.doc_type,
                                  format_query_string(&mut self.options));
                match self.document {
                    Some(ref doc) => self.client.post_body_op(&url, doc),
                    None          => self.client.post_op(&url)
                }
            }
        });
        Ok(IndexResult::from(&result.unwrap()))
    }
}

/// An ES GET operation, to get a document by ID
pub struct GetOperation<'a, 'b> {
    /// The HTTP connection
    client:   &'a mut Client,

    /// The index to load the document.
    index:    &'b str,

    /// Optional type
    doc_type: Option<&'b str>,

    /// The ID of the document.
    id:       &'b str,

    /// Optional options
    options:  Options<'b>
}

impl<'a, 'b> GetOperation<'a, 'b> {
    fn new(client:   &'a mut Client,
           index:    &'b str,
           id:       &'b str) -> GetOperation<'a, 'b> {
        GetOperation {
            client:   client,
            index:    index,
            doc_type: None,
            id:       id,
            options:  Options::new()
        }
    }

    pub fn with_all_types(&'b mut self) -> &'b mut Self {
        self.doc_type = Some("_all");
        self
    }

    pub fn with_doc_type(&'b mut self, doc_type: &'b str) -> &'b mut Self {
        self.doc_type = Some(doc_type);
        self
    }

    pub fn with_fields(&'b mut self, fields: &[&'b str]) -> &'b mut Self {
        let mut fields_str = String::new();
        for field in fields {
            fields_str.push_str(field);
            fields_str.push_str(",");
        }
        fields_str.pop();

        self.options.push(("fields", fields_str));
        self
    }

    add_option!(with_realtime, "realtime");
    add_option!(with_source, "_source");
    add_option!(with_routing, "routing");
    add_option!(with_preference, "preference");
    add_option!(with_refresh, "refresh");
    add_option!(with_version, "version");

    pub fn send(&'b mut self) -> Result<GetResult, EsError> {
        let url = format!("/{}/{}/{}{}",
                          self.index,
                          self.doc_type.unwrap(),
                          self.id,
                          format_query_string(&self.options));
        // We're ignoring status_code as all valid codes should return a value,
        // so anything else is an error.
        let (_, result) = try!(self.client.get_op(&url));
        Ok(GetResult::from(&result.unwrap()))
    }
}

/// An ES DELETE operation for a specific document
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
    fn new(client:   &'a mut Client,
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
                          format_query_string(&mut self.options));
        let (status_code, result) = try!(self.client.delete_op(&url));
        info!("DELETE OPERATION STATUS: {:?} RESULT: {:?}", status_code, result);
        match status_code {
            StatusCode::Ok =>
                Ok(DeleteResult::from(&result.unwrap())),
            _ =>
                Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        }
    }
}

struct DeleteByQueryBody {
    query: query::Query
}

// TODO: make this unnecessary
impl ToJson for DeleteByQueryBody {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("query".to_string(), self.query.to_json());
        Json::Object(d)
    }
}

enum QueryOption {
    String(String),
    Document(DeleteByQueryBody)
}

/// Delete-by-query API.
///
/// The query can be specified either as a String as a query parameter or in the
/// body using the Query DSL.
pub struct DeleteByQueryOperation<'a, 'b> {
    /// The HTTP client
    client:    &'a mut Client,

    /// The indexes to which this query apply
    indexes:   &'b [&'b str],

    /// The types to which this query applies
    doc_types: &'b [&'b str],

    /// The query itself, either in parameter or Query DSL form.
    query:     QueryOption,

    /// Optional options
    options:   Options<'b>
}

impl<'a, 'b> DeleteByQueryOperation<'a, 'b> {
    fn new(client: &'a mut Client) -> DeleteByQueryOperation<'a, 'b> {
        DeleteByQueryOperation {
            client:    client,
            indexes:   &[],
            doc_types: &[],
            query:     QueryOption::String("".to_string()),
            options:   Options::new()
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn with_doc_types(&'b mut self, doc_types: &'b [&'b str]) -> &'b mut Self {
        self.doc_types = doc_types;
        self
    }

    pub fn with_query_string(&'b mut self, qs: String) -> &'b mut Self {
        self.query = QueryOption::String(qs);
        self
    }

    pub fn with_query(&'b mut self, q: Query) -> &'b mut Self {
        self.query = QueryOption::Document(DeleteByQueryBody { query: q });
        self
    }

    add_option!(with_df, "df");
    add_option!(with_analyzer, "analyzer");
    add_option!(with_default_operator, "default_operator");
    add_option!(with_routing, "routing");
    add_option!(with_consistency, "consistency");

    pub fn send(&'a mut self) -> Result<Option<DeleteByQueryResult>, EsError> {
        let options = match &self.query {
            &QueryOption::Document(_)   => &mut self.options,
            &QueryOption::String(ref s) => {
                let opts = &mut self.options;
                opts.push(("q", s.clone()));
                opts
            }
        };
        let url = format!("/{}/{}/_query{}",
                          format_multi(&self.indexes),
                          format_multi(&self.doc_types),
                          format_query_string(options));
        let (status_code, result) = try!(match self.query {
            QueryOption::Document(ref d) => self.client.delete_body_op(&url,
                                                                       &d.to_json()),
            QueryOption::String(_)       => self.client.delete_op(&url)
        });
        info!("DELETE BY QUERY STATUS: {:?}, RESULT: {:?}", status_code, result);
        match status_code {
            StatusCode::Ok =>
                Ok(Some(DeleteByQueryResult::from(&result.unwrap()))),
            StatusCode::NotFound =>
                Ok(None),
            _  =>
                Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        }
    }
}

// Results

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

/// The result of an index operation
#[derive(Debug)]
pub struct IndexResult {
    index:    String,
    doc_type: String,
    id:       String,
    version:  i64,
    created:  bool
}

// TODO: remove the need for this
impl<'a> From<&'a Json> for IndexResult {
    fn from(r: &'a Json) -> IndexResult {
        IndexResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            version:  get_json_i64!(r, "_version"),
            created:  get_json_bool!(r, "created")
        }
    }
}

/// The result of a GET request
#[derive(Debug)]
pub struct GetResult {
    index:    String,
    doc_type: String,
    id:       String,
    version:  Option<i64>,
    found:    bool,
    source:   Option<Json>
}

impl GetResult {
    /// The result is a JSON document, this function will attempt to decode it
    /// to a struct.  If the raw JSON is required, it can accessed directly from
    /// the source field of the `GetResult` struct.
    pub fn source<T: Decodable>(self) -> Result<T, EsError> {
        match self.source {
            Some(doc) => {
                let mut decoder = Decoder::new(doc);
                Ok(try!(Decodable::decode(&mut decoder)))
            },
            None => Err(EsError::EsError("No source".to_string()))
        }
    }
}

impl<'a> From<&'a Json> for GetResult {
    fn from(r: &'a Json) -> GetResult {
        info!("GetResult FROM: {:?}", r);
        GetResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            version:  r.search("_version").map(|v| v.as_i64().unwrap()),
            found:    get_json_bool!(r, "found"),
            source:   r.search("_source").map(|source| source.clone())
        }
    }
}

/// Result of a DELETE operation
#[derive(Debug)]
pub struct DeleteResult {
    found:    bool,
    index:    String,
    doc_type: String,
    id:       String,
    version:  i64
}

// TODO remove the need for this
impl<'a> From<&'a Json> for DeleteResult {
    fn from(r: &'a Json) -> DeleteResult {
        DeleteResult {
            found:    get_json_bool!(r, "found"),
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            version:  get_json_i64!(r, "_version")
        }
    }
}

#[derive(Debug)]
pub struct DeleteByQueryShardResult {
    total:   i64,
    success: i64,
    failed:  i64
}

// TODO remove the need for this
impl<'a> From<&'a Json> for DeleteByQueryShardResult {
    fn from(r: &'a Json) -> DeleteByQueryShardResult {
        info!("DeleteByQueryShardResult from: {:?}", r);

        DeleteByQueryShardResult {
            total:   get_json_i64!(r, "total"),
            success: get_json_i64!(r, "successful"),
            failed:  get_json_i64!(r, "failed")
        }
    }
}

#[derive(Debug)]
pub struct DeleteByQueryIndexResult {
    shards: DeleteByQueryShardResult
}

impl DeleteByQueryIndexResult {
    fn successful(&self) -> bool {
        self.shards.failed == 0
    }
}

// TODO remove the need for this
impl<'a> From<&'a Json> for DeleteByQueryIndexResult {
    fn from(r: &'a Json) -> DeleteByQueryIndexResult {
        DeleteByQueryIndexResult {
            shards: DeleteByQueryShardResult::from(r.find("_shards").unwrap())
        }
    }
}

/// The result of a Delete-by-query request
#[derive(Debug)]
pub struct DeleteByQueryResult {
    indices: HashMap<String, DeleteByQueryIndexResult>
}

impl DeleteByQueryResult {
    pub fn successful(&self) -> bool {
        for dbqir in self.indices.values() {
            if !dbqir.successful() {
                return false
            }
        }
        true
    }
}

// TODO: remove the need for this
impl<'a> From<&'a Json> for DeleteByQueryResult {
    fn from(r: &'a Json) -> DeleteByQueryResult {
        info!("DeleteByQueryResult from: {:?}", r);

        let indices = r.find("_indices").unwrap().as_object().unwrap();
        let mut indices_map = HashMap::new();
        for (k, v) in indices {
            indices_map.insert(k.clone(), DeleteByQueryIndexResult::from(v));
        }
        DeleteByQueryResult {
            indices: indices_map
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    extern crate regex;

    use super::Client;
    use super::OpType;

    use super::query::Query;

    use std::env;

    use self::regex::Regex;

    use rustc_serialize::json::ToJson;

    // test setup

    fn make_client() -> Client {
        let hostname = match env::var("ES_HOST") {
            Ok(val) => val,
            Err(_)  => "localhost".to_string()
        };
        Client::new(&hostname, 9200)
    }

    #[derive(Debug, RustcDecodable, RustcEncodable)]
    struct TestDocument {
        str_field: String,
        int_field: i64
    }

    fn make_doc(int_f: i64) -> TestDocument {
        TestDocument {
            str_field: "I am a test".to_string(),
            int_field: int_f
        }
    }

    fn clean_db(client: &mut Client) {
        client.delete_by_query().with_query(Query::build_match_all().build()).send().unwrap();
    }

    // tests

    #[test]
    fn it_works() {
        let mut client = make_client();
        let result = client.version().unwrap();

        let expected_regex = Regex::new(r"^\d\.\d\.\d$").unwrap();
        assert_eq!(expected_regex.is_match(&result), true);
    }

    #[test]
    fn test_indexing() {
        env_logger::init().unwrap();

        let mut client = make_client();
        clean_db(&mut client);
        {
            let result_wrapped = client
                .index("test_idx", "test_type")
                .with_doc(&make_doc(1))
                .with_ttl(&927500)
                .send();
            info!("TEST RESULT: {:?}", result_wrapped);
            let result = result_wrapped.unwrap();
            assert_eq!(result.created, true);
            assert_eq!(result.index, "test_idx");
            assert_eq!(result.doc_type, "test_type");
            assert!(result.id.len() > 0);
            assert_eq!(result.version, 1);
        }
        {
            let delete_result = client.delete("test_idx", "test_type", "TEST_INDEXING_2").send();
            info!("DELETE RESULT: {:?}", delete_result);

            let result_wrapped = client
                .index("test_idx", "test_type")
                .with_doc(&make_doc(2))
                .with_id("TEST_INDEXING_2")
                .with_op_type(&OpType::Create)
                .send();
            let result = result_wrapped.unwrap();

            assert_eq!(result.created, true);
            assert_eq!(result.index, "test_idx");
            assert_eq!(result.doc_type, "test_type");
            assert_eq!(result.id, "TEST_INDEXING_2");
            assert!(result.version >= 1);
        }
    }

    #[test]
    fn test_get() {
        let mut client = make_client();
        clean_db(&mut client);
        {
            let doc = make_doc(3);
            client
                .index("test_idx", "test_type")
                .with_id("TEST_GETTING")
                .with_doc(&doc)
                .send().unwrap();
        }
        {
            let mut getter = client.get("test_idx", "TEST_GETTING");
            let result_wrapped = getter
                .with_doc_type("test_type")
                .send();
            info!("RESULT: {:?}", result_wrapped);
            let result = result_wrapped.unwrap();
            assert_eq!(result.id, "TEST_GETTING");

            let source:TestDocument = result.source().unwrap();
            assert_eq!(source.str_field, "I am a test");
            assert_eq!(source.int_field, 3);
        }
    }

    #[test]
    fn test_delete_by_query() {
        let mut client = make_client();
        clean_db(&mut client);

        let td1 = TestDocument {
            str_field: "TEST DOC 1".to_string(),
            int_field: 100
        };

        let td2 = TestDocument {
            str_field: "TEST DOC 2".to_string(),
            int_field: 200
        };

        client
            .index("test_idx", "test_type")
            .with_id("ABC123")
            .with_doc(&td1)
            .send().unwrap();
        client
            .index("test_idx", "test_type")
            .with_id("ABC124")
            .with_doc(&td2)
            .send().unwrap();

        let delete_result = client
            .delete_by_query()
            .with_indexes(&["test_idx"])
            .with_doc_types(&["test_type"])
            .with_query(Query::build_match("int_field".to_string(), 200.to_json())
                        .with_lenient(false)
                        .build())
            .send().unwrap();

        assert!(delete_result.unwrap().successful());

        let doc1 = client.get("test_idx", "ABC123").with_doc_type("test_type").send().unwrap();
        let doc2 = client.get("test_idx", "ABC124").with_doc_type("test_type").send().unwrap();

        assert!(doc1.found);
        assert!(!doc2.found);
    }
}
