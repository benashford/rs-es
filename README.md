# rs-es

[![Build Status](https://travis-ci.org/benashford/rs-es.svg)](https://travis-ci.org/benashford/rs-es)

An experimental ElasticSearch client for Rust.

## Background

The only existing ElasticSearch client for Rust that I could find required the use of a ZeroMQ plugin.  This project is an ongoing implementation of an ElasticSearch client via the REST API.

## Experimental

Development is ongoing, and is experimental, as such breaking changes are likely at any time.  Also, large parts of the ElasticSearch API are currently unimplemented.

## Building and installation

### [crates.io](http://crates.io)

Not yet available on [crates.io](http://crates.io), it will be as soon as a core minimal feature-set is implemented.

### Building from source

Part of the Rust code is generated automatically (`src/query.rs`), this is the implementation of ElasticSearch's [Query DSL](#the-query-dsl) which contains various conventions that would be otherwise tedious to implement manually.  Rust's macros help here, but there is still a lot of redundancy left-over so a Ruby script is used.

#### Pre-requisites

* Ruby - any relatively recent version should work, but it has been specifically tested with 2.1 and 2.2.

#### Build instructions

* Clean - `make clean`
* Build - `make`
* Run tests - `make test`

## Design goals

There are two primary goals: 1) to be a full implementation of the ElasticSearch REST API, and 2) to be idiomatic both with ElasticSearch and Rust conventions.

The second goal is more difficult to achieve than the first as there are some areas which conflict.  A small example of this is the word `type`, this is a word that refers to the type of an ElasticSearch document but it also a reserved word for definining types in Rust.  This means we cannot name a field `type` for instance, so in this library the document type is always referred to as `doc_type` instead.

## Usage guide

### The client

The `Client` wraps a single HTTP connection to a specified ElasticSearch host/port.

(At present there is no connection pooling, each client has one connection; if you need multiple connections you will need multiple clients.  This may change in the future).

```rust
let mut client = Client::new("localhost", 9200);
```

### Operations

The `Client` provides various operations, which are analogous to the various ElasticSearch APIs.

In each case the `Client` has a function which returns a builder-pattern object that allows additional options to be set.  The function itself will require mandatory parameters, everything else is on the builder (e.g. operations that require an index to be specified will have index as a parameter on the function itself).

An example of optional parameters is [`routing`](https://www.elastic.co/blog/customizing-your-document-routing).  The routing parameter can be set on operations that support it with:

```rust
op.with_routing("user123")
```

See the ElasticSearch guide for the full set of options and what they mean.

#### `index`

An implementation of the [Index API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-index_.html).

```rust
let index_op = client.index("index_name", "type_name");
```

Returned is an `IndexOperation` to add additional options.  For example, to set an ID and a TTL:

```rust
index_op.with_id("ID_VALUE").with_ttl("100d");
```

The document to be indexed has to implement the `Encodable` trait from the [`rustc-serialize`](https://github.com/rust-lang/rustc-serialize) library.  This can be achieved by either implementing or deriving that on a custom type, or by manually creating a `Json` object.

Calling `send` submits the index operation and returns an `IndexResult`:

```rust
index_op.with_doc(&document).send();
```

#### `get`

An implementation of the [Get API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-get.html).

Index and ID are mandatory, but type is optional.  Some examples:

```rust
// Finds a document of any type with the given ID
let result_1 = client.get("index_name", "ID_VALUE").send();

// Finds a document of a specific type with the given ID
let result_2 = client.get("index_name", "ID_VALUE").with_doc_type("type_name").send();
```

#### `delete`

An implementation of the [Delete API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-delete.html).

Index, type and ID are mandatory.

```rust
let result = client.delete("index_name", "type_name", "ID_VALUE").send();
```

#### `delete_by_query`

An implementation of the [Delete By Query API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-delete-by-query.html).

No fields are mandatory, but index, type and a large set of parameters are optional.  Both query strings and the [Query DSL](#the-query-dsl) are valid for this operation.

```rust
// Using a query string
let result = client.delete_by_query()
                   .with_indexes(&["index_name"])
                   .with_query_string("field:value")
                   .send();

// Using the query DSL
use rs_es::query::Query;
let result = client.delete_by_query()
                   .with_indexes(&["index_name"])
                   .with_query(Query::build_match("field", "value"))
                   .send();
```

#### `refresh`

Sends a refresh request.

```rust
// To everything
let result = client.refresh().send();

// To specific indexes
let result = client.refresh().with_indexes(&["index_name", "other_index_name"]).send();
```

#### `search_uri`

An implementation of the [Search API](https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-search.html) using query strings.

Example:

```rust
let result = client.search_uri()
                   .with_indexes(&["index_name"])
                   .with_query("field:value")
                   .send();
```

#### `search_query`

An implementation of the [Search API](https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-search.html) using the [Query DSL](#the-query-dsl).

```rust
use rs_es::query::Query;
let result = client.search_query()
                   .with_indexes(&["index_name"])
                   .with_query(Query::build_match("field", "value"))
                   .send();
```

WARNING: this doesn't actually work yet, but will do soon.

### The Query DSL

ElasticSearch offers a [rich DSL for searches](https://www.elastic.co/guide/en/elasticsearch/reference/1.x/query-dsl.html).  It is JSON based, and therefore very easy to use and composable if using from a dynamic language (e.g. [Ruby](https://github.com/elastic/elasticsearch-ruby/tree/master/elasticsearch-dsl#features-overview)); but Rust, being a staticly-typed language, things are different.  The `rs_es::query` module defines a set of builder objects which can be similarly composed to the same ends.

For example:

```rust
let query = Query::build_filtered(
                Filter::build_bool()
                    .with_must(vec![Filter::build_term("field_a".to_string(),
                                                       "value".to_json()).build(),
                                    Filter::build_range("field_b".to_string())
                                        .with_gte(5.to_json())
                                        .with_lt(10.to_json())
                                        .build()]))
                .with_query(Query::build_query_string("some value".to_string()))
                .build();
```

The resulting `Query` value can be used in the various search/query functions exposed by [the client](#the-client).  It implements [`ToJson`](http://doc.rust-lang.org/rustc-serialize/rustc_serialize/json/index.html), which in the above example would produce JSON like so:

```javascript
{
    "filtered": {
        "query": {
            "query_string": {
                "query": "some_value"
            }
        },
        "filter": {
            "bool": {
                "must": [
                    {
                        "term": {
                            "field_a": "value"
                        }
                    },
                    {
                        "range": {
                            "field_b": {
                                "gte": 5,
                                "lt": 10
                            }
                        }
                    }
                ]
            }
        }
    }
}
```

Potential future additions will remove some of the remaining verbosity for happy-path cases.

#### Experimental

This implementation of the query DSL is auto-generated and is done so in such a way to allow the generated code to change when necessary.  The template files are [query.rs.erb](templates/query.rs.erb) and [generate_query_dsl.rb](tools/generate_query_dsl.rb).  The experimental warning is recursive, it's likely that the means of generating the query DSL will change due to lessons-learnt implementing the first version.

## Unimplemented features

The ElasticSearch API is made-up of a large number of smaller APIs, the vast majority of which are not yet implemented.  So far the document and search APIs are being implemented, but still to do: index management, cluster management.

A non-exhaustive (and non-prioritised) list of unimplemented APIs:

* Search Shards API (https://www.elastic.co/guide/en/elasticsearch/reference/current/search-shards.html)
* Suggest API
* Multi-search API
* Count API
* Search Exists API
* Validate API
* Explain API
* Percolation
* More like this API
* Indices API
* cat APIs
* Cluster APIs

### TODO

0. Remove need for extraneous to_string and to_json's in the Query DSL.
1. Implementation of Search API.
2. Publish to Crates.io
3. Scan and scroll
4. Aggregations
5. Search templates (possibly)
6. Implement Update API.
7. Implement Multi Get API
8. Implement Bulk API
9. Implement Term Vectors and Multi termvectors API
10. Test coverage.
11. Performance (ensure use of persistent HTTP connections, etc.).
12. Documentation, both rustdoc and a suitable high-level write-up in this README
