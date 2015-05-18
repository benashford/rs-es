# rs-es

An experimental ElasticSearch client for Rust.

## Background

The only existing ElasticSearch client for Rust that I could find required the use of a ZeroMQ plugin.  This project is an ongoing implementation of an ElasticSearch client via the REST API.

## Unstable!

Development is ongoing, and breaking changes are likely at any time.  Also, large parts of the ElasticSearch API are currently unimplemented.

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

Currently partially implemented

TODO: complete this bit

## TODO

1. Full implementation of the Query DSL.
2. Implementation of Search API.
3. Search templates (possibly)
4. Scan and scroll
3. Implement Update API.
4. Implement Multi Get API
5. Implement Bulk API
6. Implement Term Vectors and Multi termvectors API
7. All neuances of geo-searches.
8. Test coverage.
9. Aggregations.
10. Everything else.
11. Performance (ensure use of persistent HTTP connections, etc.).
12. Documentation, both rustdoc and a suitable high-level write-up in this README
13. Wire-up to Travis
14. Publish to Crates.io
