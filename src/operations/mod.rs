/*
 * Copyright 2015 Ben Ashford
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

use std::collections::BTreeMap;
use std::collections::HashMap;

use hyper::status::StatusCode;

use rustc_serialize::{Encodable, Decodable};
use rustc_serialize::json::{Decoder, Json, ToJson};

use Client;
use error::EsError;
use format_indexes_and_types;
use format_multi;
use format_query_string;
use query::Query;
use util::StrJoin;

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
    pub fn new(client: &'a mut Client, index: &'b str, doc_type: &'b str) -> IndexOperation<'a, 'b, E> {
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
    pub fn new(client:   &'a mut Client,
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
        self.options.push(("fields", fields.iter().join(",")));
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
    pub fn new(client:   &'a mut Client,
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

struct DeleteByQueryBody<'a> {
    query: &'a Query
}

impl<'a> ToJson for DeleteByQueryBody<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("query".to_string(), self.query.to_json());
        Json::Object(d)
    }
}

enum QueryOption<'a> {
    String(String),
    Document(DeleteByQueryBody<'a>)
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
    query:     QueryOption<'b>,

    /// Optional options
    options:   Options<'b>
}

impl<'a, 'b> DeleteByQueryOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> DeleteByQueryOperation<'a, 'b> {
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

    pub fn with_query(&'b mut self, q: &'b Query) -> &'b mut Self {
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
        let url = format!("/{}/_query{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
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

/// Refresh
pub struct RefreshOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes being refreshed
    indexes: &'b [&'b str]
}

impl<'a, 'b> RefreshOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> RefreshOperation {
        RefreshOperation {
            client:  client,
            indexes: &[]
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn send(&mut self) -> Result<RefreshResult, EsError> {
        let url = format!("/{}/_refresh",
                          format_multi(&self.indexes));
        let (status_code, result) = try!(self.client.post_op(&url));
        match status_code {
            StatusCode::Ok => Ok(RefreshResult::from(&result.unwrap())),
            _              => Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        }
    }
}

/// Search API using a query string
pub struct SearchURIOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes to which this query applies
    indexes: &'b [&'b str],

    /// The types to which this query applies
    doc_types: &'b [&'b str],

    /// Optional options
    options: Options<'b>
}

/// Options for the various search_type parameters
pub enum SearchType {
    DFSQueryThenFetch,
    DFSQueryAndFetch,
    QueryThenFetch,
    QueryAndFetch
}

impl ToString for SearchType {
    fn to_string(&self) -> String {
        match self {
            &SearchType::DFSQueryThenFetch => "dfs_query_then_fetch",
            &SearchType::DFSQueryAndFetch  => "dfs_query_and_fetch",
            &SearchType::QueryThenFetch    => "query_then_fetch",
            &SearchType::QueryAndFetch     => "query_and_fetch"
        }.to_string()
    }
}

impl<'a, 'b> SearchURIOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> SearchURIOperation<'a, 'b> {
        SearchURIOperation {
            client:    client,
            indexes:   &[],
            doc_types: &[],
            options:   Options::new()
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn with_types(&'b mut self, doc_types: &'b [&'b str]) -> &'b mut Self {
        self.doc_types = doc_types;
        self
    }

    pub fn with_query(&'b mut self, qs: String) -> &'b mut Self {
        self.options.push(("q", qs));
        self
    }

    add_option!(with_df, "df");
    add_option!(with_analyzer, "analyzer");
    add_option!(with_lowercase_expanded_terms, "lowercase_expanded_terms");
    add_option!(with_analyze_wildcard, "analyze_wildcard");
    add_option!(with_default_operator, "default_operator");
    add_option!(with_lenient, "lenient");
    add_option!(with_explain, "explain");
    add_option!(with_source, "_source");
    add_option!(with_sort, "sort");
    add_option!(with_routing, "routing");
    add_option!(with_track_scores, "track_scores");
    add_option!(with_timeout, "timeout");
    add_option!(with_terminate_after, "terminate_after");
    add_option!(with_from, "from");
    add_option!(with_size, "size");
    add_option!(with_search_type, "search_type");

    pub fn with_fields(&'b mut self, fields: &[&str]) -> &'b mut Self {
        self.options.push(("fields", fields.iter().join(",")));
        self
    }

    pub fn send(&'b mut self) -> Result<SearchResult, EsError> {
        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          format_query_string(&self.options));
        info!("Searching with: {}", url);
        let (status_code, result) = try!(self.client.get_op(&url));
        info!("Search result (status: {}, result: {:?})", status_code, result);
        match status_code {
            StatusCode::Ok => Ok(SearchResult::from(&result.unwrap())),
            _              => Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        }
    }
}

struct SearchQueryOperationBody<'b> {
    /// The query
    query: Option<&'b Query>,

    /// Timeout
    timeout: Option<&'b str>,

    /// From
    from: i64,

    /// Size
    size: i64,

    /// Terminate early (marked as experimental in the ES docs)
    terminate_after: Option<i64>,

    /// Stats groups to which the query belongs
    stats: Option<Vec<String>>
}

impl<'a> ToJson for SearchQueryOperationBody<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("from".to_string(), self.from.to_json());
        d.insert("size".to_string(), self.size.to_json());
        optional_add!(d, self.query, "query");
        optional_add!(d, self.timeout, "timeout");
        optional_add!(d, self.terminate_after, "terminate_after");
        optional_add!(d, self.stats, "stats");
        Json::Object(d)
    }
}

/// Search API using a Query DSL body
pub struct SearchQueryOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes to which this query applies
    indexes: &'b [&'b str],

    /// The types to which the query applies
    doc_types: &'b [&'b str],

    /// Optionals
    options: Options<'b>,

    /// The query body
    body: SearchQueryOperationBody<'b>
}

impl <'a, 'b> SearchQueryOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> SearchQueryOperation<'a, 'b> {
        SearchQueryOperation {
            client:    client,
            indexes:   &[],
            doc_types: &[],
            options:   Options::new(),
            body:      SearchQueryOperationBody {
                query:           None,
                timeout:         None,
                from:            0,
                size:            10,
                terminate_after: None,
                stats:           None
            }
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn with_types(&'b mut self, doc_types: &'b [&'b str]) -> &'b mut Self {
        self.doc_types = doc_types;
        self
    }

    pub fn with_query(&'b mut self, query: &'b Query) -> &'b mut Self {
        self.body.query = Some(query);
        self
    }

    pub fn with_timeout(&'b mut self, timeout: &'b str) -> &'b mut Self {
        self.body.timeout = Some(timeout);
        self
    }

    pub fn with_from(&'b mut self, from: i64) -> &'b mut Self {
        self.body.from = from;
        self
    }

    pub fn with_size(&'b mut self, size: i64) -> &'b mut Self {
        self.body.size = size;
        self
    }

    pub fn with_terminate_after(&'b mut self, terminate_after: i64) -> &'b mut Self {
        self.body.terminate_after = Some(terminate_after);
        self
    }

    pub fn with_stats<S>(&'b mut self, stats: &[S]) -> &'b mut Self
        where S: ToString
    {
        self.body.stats = Some(stats.iter().map(|s| s.to_string()).collect());
        self
    }

    add_option!(with_routing, "routing");
    add_option!(with_search_type, "search_type");
    add_option!(with_query_cache, "query_cache");

    pub fn send(&'b mut self) -> Result<SearchResult, EsError> {
        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          format_query_string(&self.options));
        let (status_code, result) = try!(self.client.post_body_op(&url, &self.body.to_json()));
        match status_code {
            StatusCode::Ok => Ok(SearchResult::from(&result.unwrap())),
            _              => Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        }
    }
}

// Results

// Result helpers

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

macro_rules! get_json_f64 {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_f64)
    }
}

fn decode_json<T: Decodable>(doc: Json) -> Result<T, EsError> {
    Ok(try!(Decodable::decode(&mut Decoder::new(doc))))
}

/// Shared struct for operations that include counts of success/failed shards
#[derive(Debug, RustcDecodable)]
pub struct ShardCountResult {
    pub total:      i64,
    pub successful: i64,
    pub failed:     i64
}

/// The result of an index operation
#[derive(Debug)]
pub struct IndexResult {
    pub index:    String,
    pub doc_type: String,
    pub id:       String,
    pub version:  i64,
    pub created:  bool
}

/// This is required because the JSON keys do not match the struct
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
    pub index:    String,
    pub doc_type: String,
    pub id:       String,
    pub version:  Option<i64>,
    pub found:    bool,
    pub source:   Option<Json>
}

impl GetResult {
    /// The result is a JSON document, this function will attempt to decode it
    /// to a struct.  If the raw JSON is required, it can accessed directly from
    /// the source field of the `GetResult` struct.
    pub fn source<T: Decodable>(self) -> Result<T, EsError> {
        match self.source {
            Some(doc) => decode_json(doc),
            None      => Err(EsError::EsError("No source".to_string()))
        }
    }
}

/// This is required because the JSON keys do not match the struct
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
    pub found:    bool,
    pub index:    String,
    pub doc_type: String,
    pub id:       String,
    pub version:  i64
}

/// This is required because the JSON keys do not match the struct
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
pub struct DeleteByQueryIndexResult {
    pub shards: ShardCountResult
}

impl DeleteByQueryIndexResult {
    fn successful(&self) -> bool {
        self.shards.failed == 0
    }
}

// Required because of change in names of keys
impl<'a> From<&'a Json> for DeleteByQueryIndexResult {
    fn from(r: &'a Json) -> DeleteByQueryIndexResult {
        info!("Parsing DeleteByQueryIndexResult: {:?}", r);
        DeleteByQueryIndexResult {
            shards: decode_json(r.find("_shards").unwrap().clone()).unwrap()
        }
    }
}

/// The result of a Delete-by-query request
#[derive(Debug)]
pub struct DeleteByQueryResult {
    pub indices: HashMap<String, DeleteByQueryIndexResult>
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

// Required because of JSON structure and keys
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

/// Result of a refresh request
pub struct RefreshResult {
    pub shards: ShardCountResult
}

impl<'a> From<&'a Json> for RefreshResult {
    fn from(r: &'a Json) -> RefreshResult {
        RefreshResult {
            shards: decode_json(r.find("_shards").unwrap().clone()).unwrap()
        }
    }
}

#[derive(Debug)]
pub struct SearchHitsHitsResult {
    pub index:    String,
    pub doc_type: String,
    pub id:       String,
    pub score:    f64,
    pub source:   Option<Json>,
    pub fields:   Option<Json>
}

impl SearchHitsHitsResult {
    /// Get the source document as a struct, the raw JSON version is available
    /// directly from the source field
    pub fn source<T: Decodable>(self) -> Result<T, EsError> {
        match self.source {
            Some(source) => decode_json(source),
            None         => Err(EsError::EsError("No source field".to_string()))
        }
    }
}

impl<'a> From<&'a Json> for SearchHitsHitsResult {
    fn from(r: &'a Json) -> SearchHitsHitsResult {
        SearchHitsHitsResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            score:    get_json_f64!(r, "_score"),
            source:   r.find("_source").map(|s| s.clone()),
            fields:   r.find("fields").map(|s| s.clone())
        }
    }
}

pub struct SearchHitsResult {
    pub total: i64,
    pub hits:  Vec<SearchHitsHitsResult>
}

impl<'a> From<&'a Json> for SearchHitsResult {
    fn from(r: &'a Json) -> SearchHitsResult {
        SearchHitsResult {
            total: get_json_i64!(r, "total"),
            hits:  r.find("hits")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|j| SearchHitsHitsResult::from(j))
                .collect()
        }
    }
}

pub struct SearchResult {
    pub shards: ShardCountResult,
    pub hits:   SearchHitsResult
}

impl<'a> From<&'a Json> for SearchResult {
    fn from(r: &'a Json) -> SearchResult {
        SearchResult {
            shards: decode_json(r.find("_shards")
                                .unwrap()
                                .clone()).unwrap(),
            hits:   SearchHitsResult::from(r.find("hits")
                                           .unwrap())
        }
    }
}
