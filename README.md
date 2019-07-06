# rs-es

[![Build Status](https://travis-ci.org/benashford/rs-es.svg?branch=master)](https://travis-ci.org/benashford/rs-es)
[![](http://meritbadge.herokuapp.com/rs-es)](https://crates.io/crates/rs-es)
[![](https://img.shields.io/crates/d/rs-es.svg)](https://crates.io/crates/rs-es)
[![](https://img.shields.io/crates/dv/rs-es.svg)](https://crates.io/crates/rs-es)
[![](https://docs.rs/rs-es/badge.svg)](https://docs.rs/rs-es/)
[![Dependency Status](https://dependencyci.com/github/benashford/rs-es/badge)](https://dependencyci.com/github/benashford/rs-es)

## Introduction

An ElasticSearch client for Rust via the REST API.  Targetting ElasticSearch 2.0 and higher.

Development is ongoing, and is experimental, as such breaking changes are likely at any time.  Also, large parts of the ElasticSearch API are currently unimplemented.

Not every feature and every option is implemented, this README and the documentation describe what is available.  For any errors, omissions, etc., issues and pull requests are welcome.

## Documentation

[Full documentation for `rs-es`](http://benashford.github.io/rs-es/rs_es/index.html).

## Building and installation

Version `0.11.0` requires Rust `1.31.0` or higher.

### [crates.io](http://crates.io)

Available from [crates.io](https://crates.io/crates/rs-es).

## Design goals

There are two primary goals: 1) to be a full implementation of the ElasticSearch REST API, and 2) to be idiomatic both with ElasticSearch and Rust conventions.

The second goal is more difficult to achieve than the first as there are some areas which conflict.  A small example of this is the word `type`, this is a word that refers to the type of an ElasticSearch document but it also a reserved word for definining types in Rust.  This means we cannot name a field `type` for instance, so in this library the document type is always referred to as `doc_type` instead.

### Alternatives

For an ElasticSearch client for Rust that takes a different approach, allowing free-form query creation, take a look at [`elasticsearch-rs`](https://github.com/KodrAus/elasticsearch-rs).

## Usage guide

### The client

The `Client` wraps a single HTTP connection to a specified ElasticSearch host/port.

(At present there is no connection pooling, each client has one connection; if you need multiple connections you will need multiple clients.  This may change in the future).

```rust,no_run
use rs_es::Client;

let mut client = Client::init("http://localhost:9200");
```

### Operations

The `Client` provides various operations, which are analogous to the various ElasticSearch APIs.

In each case the `Client` has a function which returns a builder-pattern object that allows additional options to be set.  The function itself will require mandatory parameters, everything else is on the builder (e.g. operations that require an index to be specified will have index as a parameter on the function itself).

An example of optional parameters is [`routing`](https://www.elastic.co/blog/customizing-your-document-routing). The routing parameter can be set on operations that support it with:

```rust,ignore
op.with_routing("user123")
```

See the ElasticSearch guide for the full set of options and what they mean.

#### `index`

An implementation of the [Index API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-index_.html).

```rust,ignore
let index_op = client.index("index_name", "type_name");
```

Returned is an `IndexOperation` to add additional options.  For example, to set an ID and a TTL:

```rust,ignore
index_op.with_id("ID_VALUE").with_ttl("100d");
```

The document to be indexed has to implement the `Serialize` trait from the [`serde`](https://github.com/serde-rs/serde) library.  This can be achieved by either implementing or deriving that on a custom type, or by manually creating a `Value` object.

Calling `send` submits the index operation and returns an `IndexResult`:

```rust,ignore
index_op.with_doc(&document).send();
```

#### `get`

An implementation of the [Get API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-get.html).

Index and ID are mandatory, but type is optional.  Some examples:

```rust,ignore
// Finds a document of any type with the given ID
let result_1 = client.get("index_name", "ID_VALUE").send();

// Finds a document of a specific type with the given ID
let result_2 = client.get("index_name", "ID_VALUE").with_doc_type("type_name").send();
```

#### `delete`

An implementation of the [Delete API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-delete.html).

Index, type and ID are mandatory.

```rust,ignore
let result = client.delete("index_name", "type_name", "ID_VALUE").send();
```

#### `refresh`

Sends a refresh request.

```rust,no_run
use rs_es::Client;

let mut client = Client::init("http://localhost:9200").expect("connection failed");
// To everything
let result = client.refresh().send();

// To specific indexes
let result = client.refresh().with_indexes(&["index_name", "other_index_name"]).send();
```

#### `search_uri`

An implementation of the [Search API](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html) using query strings.

Example:

```rust,no_run
use rs_es::Client;

let mut client = Client::init("http://localhost:9200").expect("connection failed");
let result = client.search_uri()
                   .with_indexes(&["index_name"])
                   .with_query("field:value")
                   .send::<String>();
```

#### `search_query`

An implementation of the [Search API](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html) using the [Query DSL](#the-query-dsl).

```rust,no_run
use rs_es::Client;
use rs_es::query::Query;

let mut client = Client::init("http://localhost:9200").expect("connection failed");
let result = client.search_query()
                   .with_indexes(&["index_name"])
                   .with_query(&Query::build_match("field", "value").build())
                   .send::<String>();
```

A search query also supports [scan and scroll](#scan-and-scroll), [sorting](#sorting), and [aggregations](#aggregations).

#### `count_uri`

An implementation of the [Count API](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-count.html) using query strings.

Example:

```rust,no_run
use rs_es::Client;

let mut client = Client::init("http://localhost:9200").expect("connection failed");
let result = client.count_uri()
                   .with_indexes(&["index_name"])
                   .with_query("field:value")
                   .send();
```

#### `count_query`

An implementation of the [Count API](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-count.html) using the [Query DSL](#the-query-dsl).

```rust,no_run
use rs_es::Client;
use rs_es::query::Query;

let mut client = Client::init("http://localhost:9200").expect("connection failed");
let result = client.count_query()
                   .with_indexes(&["index_name"])
                   .with_query(&Query::build_match("field", "value").build())
                   .send();
```

#### `bulk`

An implementation of the [Bulk API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html).  This is the preferred way of indexing (or deleting, when Delete-by-Query is removed) many documents.

```rust,ignore
use rs_es::operations::bulk::Action;

let result = client.bulk(&vec![Action::index(document1),
                               Action::index(document2).with_id("id")]);
```

In this case the document can be anything that implements `ToJson`.

### Sorting

Sorting is supported on all forms of search (by query or by URI), and related operations (e.g. scan and scroll).

```rust,no_run
use rs_es::Client;
use rs_es::query::Query;
use rs_es::operations::search::{Order, Sort, SortBy, SortField};

let mut client = Client::init("http://localhost:9200").expect("connection failed");
let result = client.search_query()
                   .with_query(&Query::build_match_all().build())
                   .with_sort(&Sort::new(vec![
		       SortBy::Field(SortField::new("fieldname", Some(Order::Desc)))
		   ]))
                   .send::<String>();
```

This is quite unwieldy for simple cases, although it does support the more [exotic combinations that ElasticSearch supports](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-sort.html#_geo_distance_sorting); so there are also a number of convenience functions for the more simple cases, e.g. sorting by a field in ascending order:

```rust,ignore
// Omitted the rest of the query
.with_sort(&Sort::field("fieldname"))
```

### Results

Each of the defined operations above returns a result.  Specifically this is a struct that is a direct mapping to the JSON that ElasticSearch returns.

One of the most common return types is that from the search operations, this too mirrors the JSON that ElasticSearch returns.  The top-level contains two fields, `shards` returns counts of successful/failed operations per shard, and `hits` contains the search results.  These results are in the form of another struct that has two fields `total` the total number of matching results; and `hits` which is a vector of individual results.

The individual results contain meta-data for each hit (such as the score) as well as the source document (unless the query set the various options which would disable or alter this).

The type of the source document can be anything that implemented [`Deserialize`](https://serde-rs.github.io/serde/serde/de/trait.Deserialize.html).  ElasticSearch search may return many different types of document, it also doesn't (by default) enforce any schema, this together means the structure of a returned document may need to be validated before being deserialised.  In this case a search result can return a [`Value`](http://serde-rs.github.io/json/serde_json/value/enum.Value.html) from that data can be extracted and/or converted to other structures.

### The Query DSL

ElasticSearch offers a [rich DSL for searches](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html).  It is JSON based, and therefore very easy to use and composable if using from a dynamic language (e.g. [Ruby](https://github.com/elastic/elasticsearch-ruby/tree/master/elasticsearch-dsl#features-overview)); but Rust, being a staticly-typed language, things are different.  The `rs_es::query` module defines a set of builder objects which can be similarly composed to the same ends.

For example:

```rust
use rs_es::query::Query;

let query = Query::build_bool()
    .with_must(vec![Query::build_term("field_a",
                                      "value").build(),
                    Query::build_range("field_b")
                          .with_gte(5)
                          .with_lt(10)
                          .build()])
    .build();
```

The resulting `Query` value can be used in the various search/query functions exposed by [the client](#the-client).

The implementation makes much use of [conversion traits](http://benashford.github.io/blog/2015/05/24/rust-traits-for-developer-friendly-libraries/) which are used to keep a lid on the verbosity of using such a builder pattern.

### Scan and scroll

When working with large result sets that need to be loaded from an ElasticSearch query, the most efficient way is to use [scan and scroll](https://www.elastic.co/guide/en/elasticsearch/guide/current/scan-scroll.html).  This is preferred to simple pagination by setting the `from` option in a search as it will keep resources open server-side allowing the next page to literally carry-on from where it was, rather than having to execute additional queries.  The downside to this is that it does require more memory/open file-handles on the server, which could go wrong if there were many un-finished scrolls; for this reason, ElasticSearch recommends a short time-out for such operations, after which it will close all resources whether the client has finished or not, the client is responsible to fetch the next page within the time-out.

To use scan and scroll, begin with a [search query](#search_query) request, but instead of calling `send` call `scan`:

```rust,ignore
let scan = client.search_query()
                 .with_indexes(&["index_name"])
                 .with_query(Query::build_match("field", "value").build())
                 .scan(Duration::minutes(1))
                 .unwrap();
```

(Disclaimer: any use of `unwrap` in this or other example is for the purposes of brevity, obviously real code should handle errors in accordance to the needs of the application.)

Then `scroll` can be called multiple times to fetch each page.  Finally `close` will tell ElasticSearch the scan has finished and it can close any open resources.

```rust,ignore
let first_page = scan.scroll(&mut client);
// omitted - calls of subsequent pages
scan.close(&mut client).unwrap();
```

The result of the call to `scan` does not include a reference to the client, hence the need to pass in a reference to the client in subsequent calls.  The advantage of this is that that same client could be used for actions based on each `scroll`.

#### Scan and scroll with an iterator

Also supported is an iterator which will scroll through a scan.

```rust,ignore
let scan_iter = scan.iter(&mut client);
```

The iterator will include a mutable reference to the client, so the same client cannot be used concurrently.  However the iterator will automatically call `close` when it is dropped, this is so the consumer of such an iterator can use iterator functions like `take` or `take_while` without having to decide when to call `close`.

The type of each value returned from the iterator is `Result<SearchHitsHitsResult, EsError>`.  If an error is returned than it must be assumed the iterator is closed.  The type `SearchHitsHitsResult` is the same as returned in a normal search (the verbose name is intended to mirror the structure of JSON returned by ElasticSearch).

### Aggregations

Experimental support for aggregations is also supported.

```rust,ignore
client.search_query().with_indexes(&[index_name]).with_aggs(&aggs).send();
```

Where `aggs` is a `rs_es::operations::search::aggregations::Aggregations`, for convenience sake conversion traits are implemented for common patterns; specifically the tuple `(&str, Aggregation)` for a single aggregation, and `Vec<(&str, Aggregation)>` for multiple aggregations.

Bucket aggregations (i.e. those that define a bucket that can contain sub-aggregations) can also be specified as a tuple `(Aggregation, Aggregations)`.

```rust
use rs_es::operations::search::aggregations::Aggregations;
use rs_es::operations::search::aggregations::bucket::{Order, OrderKey, Terms};
use rs_es::operations::search::aggregations::metrics::Min;

let aggs = Aggregations::from(("str",
                               (Terms::field("str_field").with_order(Order::asc(OrderKey::Term)),
                                Aggregations::from(("int",
                                                    Min::field("int_field"))))));

```

The above would, when used within a `search_query` operation, generate a JSON fragment within the search request:

```json
"str": {
    "terms": {
        "field": "str_field",
        "order": {"_term": "asc"}
    },
    "aggs": {
        "int": {
            "field": "int_field"
        }
    }
}
```

The majority, but not all aggregations are currently supported.  See the [documentation of the aggregations package](http://benashford.github.io/rs-es/rs_es/operations/search/aggregations/index.html) for details.

For example, to get the a reference to the result of the Terms aggregation called `str` (see above):

```rust,ignore
let terms_result = result.aggs_ref()
    .unwrap()
    .get("str")
    .unwrap()
    .as_terms()
    .unwrap()
```

EXPERIMENTAL: the structure of results may change as it currently feels quite cumbersome.

## Unimplemented features

The ElasticSearch API is made-up of a large number of smaller APIs, the vast majority of which are not yet implemented, although the most frequently used ones (searching, indexing, etc.) are.

### Some, non-exhaustive, specific TODOs

1. Add a CONTRIBUTING.md
2. Handling API calls that don't deal with JSON objects.
3. Documentation.
4. Potentially: Concrete (de)serialization for aggregations and aggregation results
5. Metric aggregations can have an empty body (check: all or some of them?) when used as a sub-aggregation underneath certain other aggregations.
6. Performance (ensure use of persistent HTTP connections, etc.).
7. All URI options are just String (or things that implement ToString), sometimes the values will be arrays that should be coerced into various formats.
8. Check type of "timeout" option on Search...

## Licence

```text
   Copyright 2015-2017 Ben Ashford

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
```
