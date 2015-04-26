#![crate_type = "lib"]
#![crate_name = "rs_es"]

#![feature(collections)]
#![feature(convert)]

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

use rustc_serialize::json;
use rustc_serialize::json::{Json, ToJson};

use query::Query;

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

fn format_multi(parts: &Vec<String>) -> String {
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

fn do_req<'a>(rb:   hyper::client::RequestBuilder<'a, &str>,
              body: Option<&'a str>) -> Result<Option<Json>, EsError> {
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
                Ok(json) => Ok(Some(json)),
                Err(e)   => Err(EsError::from(e))
            },
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
    ($n:ident,$cn:ident) => {
        fn $n(&mut self, url: &str, body: Option<&Json>) -> Result<Option<Json>, EsError> {
            info!("Doing {} on {} with {:?}", stringify!($n), url, body);
            match body {
                Some(json) => {
                    let json_string = json::encode(json).unwrap();
                    do_req(self.http_client.$cn(url), Some(json_string.as_str()))
                },
                None => {
                    do_req(self.http_client.$cn(url), None)
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

    es_op!(get_op, get);
    es_op!(post_op, post);
    es_op!(put_op, put);
    es_op!(delete_op, delete);

    pub fn version(&mut self) -> Result<String, EsError> {
        let url = self.get_base_url();
        let json = try!(self.get_op(url.as_str(), None)).unwrap();
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

    pub fn get<'a>(&'a mut self,
                   index: &'a str,
                   id:    &'a str) -> GetOperation {
        GetOperation::new(self, index, id)
    }

    pub fn delete<'a>(&'a mut self,
                      index:    &'a str,
                      doc_type: &'a str,
                      id:       &'a str) -> DeleteOperation {
        DeleteOperation::new(self, index, doc_type, id)
    }

    pub fn delete_by_query<'a>(&'a mut self) -> DeleteByQueryOperation {
        DeleteByQueryOperation::new(self)
    }
}

// Specific operations

type Options = Vec<(&'static str, String)>;

pub enum OpType {
    Create
}

impl ToString for OpType {
    fn to_string(&self) -> String {
        "create".to_string()
    }
}

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
                self.client.put_op(url.as_str(), match self.document {
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
                self.client.post_op(url.as_str(), match self.document {
                    Some(ref doc) => Some(doc),
                    None          => None
                })
            }
        });
        Ok(IndexResult::from(&result.unwrap()))
    }
}

pub struct GetOperation<'a> {
    client:   &'a mut Client,
    index:    &'a str,
    doc_type: Option<&'a str>,
    id:       &'a str,
    options:  Options
}

impl<'a> GetOperation<'a> {
    fn new(client:   &'a mut Client,
           index:    &'a str,
           id:       &'a str) -> GetOperation<'a> {
        GetOperation {
            client:   client,
            index:    index,
            doc_type: None,
            id:       id,
            options:  Options::new()
        }
    }

    pub fn with_all_types(&'a mut self) -> &'a mut GetOperation {
        self.doc_type = Some("_all");
        self
    }

    pub fn with_doc_type(&'a mut self, doc_type: &'a str) -> &'a mut GetOperation {
        self.doc_type = Some(doc_type);
        self
    }

    pub fn with_fields(&'a mut self, fields: &[&'a str]) -> &'a mut GetOperation {
        let mut fields_str = String::new();
        for field in fields {
            fields_str.push_str(field);
            fields_str.push_str(",");
        }
        fields_str.pop();

        self.options.push(("fields", fields_str));
        self
    }

    add_option!(with_realtime, "realtime", GetOperation);
    add_option!(with_source, "_source", GetOperation);
    add_option!(with_routing, "routing", GetOperation);
    add_option!(with_preference, "preference", GetOperation);
    add_option!(with_refresh, "refresh", GetOperation);
    add_option!(with_version, "version", GetOperation);

    pub fn send(&'a mut self) -> Result<GetResult, EsError> {
        let url = format!("{}{}/{}/{}{}",
                          self.client.get_base_url(),
                          self.index,
                          self.doc_type.unwrap(),
                          self.id,
                          format_query_string(&mut self.options));
        let result = try!(self.client.get_op(url.as_str(), None));
        Ok(GetResult::from(&result.unwrap()))
    }
}

pub struct DeleteOperation<'a> {
    client:   &'a mut Client,
    index:    &'a str,
    doc_type: &'a str,
    id:       &'a str,
    options:  Options
}

impl<'a> DeleteOperation<'a> {
    fn new(client:   &'a mut Client,
           index:    &'a str,
           doc_type: &'a str,
           id:       &'a str) -> DeleteOperation<'a> {
        DeleteOperation {
            client:   client,
            index:    index,
            doc_type: doc_type,
            id:       id,
            options:  Options::new()
        }
    }

    add_option!(with_version, "version", DeleteOperation);
    add_option!(with_routing, "routing", DeleteOperation);
    add_option!(with_parent, "parent", DeleteOperation);
    add_option!(with_consistency, "consistency", DeleteOperation);
    add_option!(with_refresh, "refresh", DeleteOperation);
    add_option!(with_timeout, "timeout", DeleteOperation);

    pub fn send(&'a mut self) -> Result<DeleteResult, EsError> {
        let url = format!("{}{}/{}/{}{}",
                          self.client.get_base_url(),
                          self.index,
                          self.doc_type,
                          self.id,
                          format_query_string(&mut self.options));
        let result = try!(self.client.delete_op(url.as_str(), None));
        info!("DELETE OPERATION RESULT: {:?}", result);
        Ok(DeleteResult::from(&result.unwrap()))
    }
}

struct DeleteByQueryBody {
    query: query::Query
}

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

macro_rules! add_to_vec_option {
    ($n:ident, $c:ident, $t:ident) => {
        pub fn $n(&'a mut self, val: String) -> &'a mut $t {
            self.$c.push(val);
            self
        }
    }
}

pub struct DeleteByQueryOperation<'a> {
    client:    &'a mut Client,
    indexes:   Vec<String>,
    doc_types: Vec<String>,
    query:     QueryOption,
    options:   Options
}

impl<'a> DeleteByQueryOperation<'a> {
    fn new(client: &'a mut Client) -> DeleteByQueryOperation<'a> {
        DeleteByQueryOperation {
            client:    client,
            indexes:   Vec::with_capacity(1),
            doc_types: Vec::with_capacity(1),
            query:     QueryOption::String("".to_string()),
            options:   Options::new()
        }
    }

    add_to_vec_option!(add_index, indexes, DeleteByQueryOperation);
    add_to_vec_option!(add_doc_type, doc_types, DeleteByQueryOperation);

    pub fn with_query_string(&'a mut self, qs: String) -> &'a mut DeleteByQueryOperation {
        self.query = QueryOption::String(qs);
        self
    }

    pub fn with_query(&'a mut self, q: Query) -> &'a mut DeleteByQueryOperation {
        self.query = QueryOption::Document(DeleteByQueryBody { query: q });
        self
    }

    add_option!(with_df, "df", DeleteByQueryOperation);
    add_option!(with_analyzer, "analyzer", DeleteByQueryOperation);
    add_option!(with_default_operator, "default_operator", DeleteByQueryOperation);
    add_option!(with_routing, "routing", DeleteByQueryOperation);
    add_option!(with_consistency, "consistency", DeleteByQueryOperation);

    pub fn send(&'a mut self) -> Result<DeleteByQueryResult, EsError> {
        let options = match &self.query {
            &QueryOption::Document(_)   => &mut self.options,
            &QueryOption::String(ref s) => {
                let opts = &mut self.options;
                opts.push(("q", s.clone()));
                opts
            }
        };
        let url = format!("{}{}/{}/_query{}",
                          self.client.get_base_url(),
                          format_multi(&self.indexes),
                          format_multi(&self.doc_types),
                          format_query_string(options));
        let result = try!(match self.query {
            QueryOption::Document(ref d) => self.client.delete_op(url.as_str(),
                                                                  Some(&d.to_json())),
            QueryOption::String(_)       => self.client.delete_op(url.as_str(),
                                                                  None)
        });
        info!("DELETE BY QUERY RESULT: {:?}", result);
        Ok(DeleteByQueryResult::from(&result.unwrap()))
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
    pub fn source<T: From<Json>>(self) -> T {
        T::from(self.source.unwrap())
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

#[derive(Debug)]
pub struct DeleteResult {
    found:    bool,
    index:    String,
    doc_type: String,
    id:       String,
    version:  i64
}

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

impl<'a> From<&'a Json> for DeleteByQueryIndexResult {
    fn from(r: &'a Json) -> DeleteByQueryIndexResult {
        DeleteByQueryIndexResult {
            shards: DeleteByQueryShardResult::from(r.find("_shards").unwrap())
        }
    }
}

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
    use super::query::Query::{MatchAll};

    use std::collections::BTreeMap;

    use self::regex::Regex;

    use rustc_serialize::json::{Json, ToJson};

    // test setup

    fn make_client() -> Client {
        Client::new("localhost".to_string(), 9200)
    }

    struct TestDocument {
        str_field: String,
        int_field: i64
    }

    impl ToJson for TestDocument {
        fn to_json(&self) -> Json {
            let mut d = BTreeMap::new();
            d.insert("str_field".to_string(), self.str_field.to_json());
            d.insert("int_field".to_string(), self.int_field.to_json());

            Json::Object(d)
        }
    }

    impl From<Json> for TestDocument {
        fn from(r: Json) -> TestDocument {
            TestDocument {
                str_field: get_json_string!(r, "str_field"),
                int_field: get_json_i64!(r, "int_field")
            }
        }
    }

    fn make_doc(int_f: i64) -> TestDocument {
        TestDocument {
            str_field: "I am a test".to_string(),
            int_field: int_f
        }
    }

    fn clean_db(client: &mut Client) {
        client.delete_by_query().with_query(MatchAll).send().unwrap();
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
        clean_db(&mut client);
        {
            let mut indexer = client.index("test_idx", "test_type");
            let doc = make_doc(1);
            let result_wrapped = indexer.with_doc(&doc).with_ttl(&927500).send();
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

            let mut indexer = client.index("test_idx", "test_type");
            let doc = make_doc(2);
            let result_wrapped = indexer
                .with_doc(&doc)
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

            let source:TestDocument = result.source();
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

        client.index("test_idx", "test_type").with_id("ABC123").with_doc(&td1).send().unwrap();
        client.index("test_idx", "test_type").with_id("ABC124").with_doc(&td2).send().unwrap();

        let delete_result = client
            .delete_by_query()
            .add_index("test_idx".to_string())
            .add_doc_type("test_type".to_string())
            .with_query(Query::build_match("int_field".to_string(), 200.to_json())
                        .with_lenient(false)
                        .build())
            .send().unwrap();

        assert!(delete_result.successful());

        let doc1 = client.get("test_idx", "ABC123").with_doc_type("test_type").send().unwrap();
        let doc2 = client.get("test_idx", "ABC124").with_doc_type("test_type").send().unwrap();

        assert!(doc1.found);
        assert!(!doc2.found);
    }
}
