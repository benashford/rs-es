# rs-es

[![Build Status](https://travis-ci.org/benashford/rs-es.svg?branch=master)](https://travis-ci.org/benashford/rs-es)
[![](http://meritbadge.herokuapp.com/rs-es)](https://crates.io/crates/rs-es)

An experimental ElasticSearch client for Rust via the REST API.

Development is ongoing, and is experimental, as such breaking changes are likely at any time.  Also, large parts of the ElasticSearch API are currently unimplemented.

Currently being developed and tested against ElasticSearch 1.6.x, it will almost certainly not work with other versions.

## Documentation

[Full documentation for `rs-es`](http://benashford.github.io/rs-es/rs_es/index.html).  The rest of this document consists of an introduction.

## Building and installation

### [crates.io](http://crates.io)

Available from [crates.io](https://crates.io/crates/rs-es).

### Building from source

Part of the Rust code is generated automatically (`src/query.rs`), this is the implementation of ElasticSearch's [Query DSL](#the-query-dsl) which contains various conventions that would be otherwise tedious to implement manually.  Rust's macros help here, but there is still a lot of redundancy left-over so a Ruby script is used.

#### Pre-requisites

* Ruby - any relatively recent version should work, but it has been specifically tested with 2.1 and 2.2.

#### Build instructions

The code-generation is integreated into Cargo, so usual `cargo test` commands will do the right thing.

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
                   .with_query(Query::build_match("field", "value").build())
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
                   .with_query(Query::build_match("field", "value").build())
                   .send();
```

#### `bulk`

An implementation of the [Bulk API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html).  This is the preferred way of indexing (or deleting, when Delete-by-Query is removed) many documents.

```rust
use rs_es::operations::bulk::Action;
let result = client.bulk(&vec![Action::index(document1),
                               Action::index(document2).with_id("id")]);
```

In this case the document can be anything that implements `ToJson`.

### Sorting

Sorting is supported on all forms of search (by query or by URI), and related operations (e.g. scan and scroll).

```rust
use rs_es::query::Query;
let result = client.search_query()
                   .with_query(Query::match_all().build())
                   .with_sort(&Sort::new(vec![SortField::new("fieldname", Order::Desc)]))
                   .send();
```

This is quite unwieldy for simple cases, although it does support the more [exotic combinations that ElasticSearch supports](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-sort.html#_geo_distance_sorting); so there are also a number of convenience functions for the more simple cases, e.g. sorting by a field in ascending order:

```rust
// Omitted the rest of the query
.with_sort(&Sort::field("fieldname"))
```

### Results

Each of the defined operations above returns a result.  Specifically this is a struct that is a direct mapping to the JSON that ElasticSearch returns.

One of the most common return types is that from the search operations, this too mirrors the JSON that ElasticSearch returns.  The top-level contains two fields, `shards` returns counts of successful/failed operations per shard, and `hits` contains the search results.  These results are in the form of another struct that has two fields `total` the total number of matching results; and `hits` which is a vector of individual results.

The individual results contain meta-data for each hit (such as the score) as well as the source document (unless the query set the various options which would disable or alter this).

The type of the source document is [`Json`](http://doc.rust-lang.org/rustc-serialize/rustc_serialize/json/enum.Json.html).  It is up to the caller to transform this into the required format.  This flexibility is desirable because an ElasticSearch search may return many different types of document, it also doesn't (by default) enforce any schema, this together means the structure of a returned document may need to be validated before being deserialised.

However, for cases when the caller is confident that the document matches a known structure (and is willing to handle any errors when that is not the case), a convenience function is available on the individual search hit which will decode the `Json` object into any type that implements [`Decodable`](http://doc.rust-lang.org/rustc-serialize/rustc_serialize/trait.Decodable.html).  See the `rustc-serialize` documentation for more details, but the simplest way of defining such a struct may be to derive [`RustcDecodable`](http://doc.rust-lang.org/rustc-serialize/rustc_serialize/json/index.html#using-autoserialization).

Finally, there is a method to decode a full set of hits.

##### Examples

First, with Json source documents:

```rust
let result = client.search_query().with_query(query).send();

// An iterator over the Json source documents
for hit in result.hits.hits {
    println!("Json document: {:?}", hits.source.unwrap());
}
```

Second, de-serialising to a struct:

```rust
// Define the struct
#[derive(Debug, RustcDecodable)]
struct DocType {
    example_field: String,
    other_field:   Vec<i64>
}

// In a function later...
let result = client.search_query().with_query(query).send();

for hit in result.hits.hits {
    let document:DocType = hit.source().unwrap(); // Warning, will panic if document doesn't match type
    println!("DocType document: {:?}", document);
}
```

Or alternatively:

```rust
let result = client.search_query().with_query(query).send();

let hits:Vec<DocType> = result.hits.hits().unwrap();
```

### The Query DSL

ElasticSearch offers a [rich DSL for searches](https://www.elastic.co/guide/en/elasticsearch/reference/1.x/query-dsl.html).  It is JSON based, and therefore very easy to use and composable if using from a dynamic language (e.g. [Ruby](https://github.com/elastic/elasticsearch-ruby/tree/master/elasticsearch-dsl#features-overview)); but Rust, being a staticly-typed language, things are different.  The `rs_es::query` module defines a set of builder objects which can be similarly composed to the same ends.

For example:

```rust
let query = Query::build_filtered(
                Filter::build_bool()
                    .with_must(vec![Filter::build_term("field_a",
                                                       "value").build(),
                                    Filter::build_range("field_b")
                                        .with_gte(5)
                                        .with_lt(10)
                                        .build()]))
                .with_query(Query::build_query_string("some value").build())
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

The implementation makes much use of [conversion traits](http://benashford.github.io/blog/2015/05/24/rust-traits-for-developer-friendly-libraries/) which are used to keep a lid on the verbosity of using such a builder pattern.

#### Experimental

This implementation of the query DSL is auto-generated and is done so in such a way to allow the generated code to change when necessary.  The template files are [query.rs.erb](templates/query.rs.erb) and [generate_query_dsl.rb](tools/generate_query_dsl.rb).  The experimental warning is recursive, it's likely that the means of generating the query DSL will change due to lessons-learnt implementing the first version.

### Scan and scroll

When working with large result sets that need to be loaded from an ElasticSearch query, the most efficient way is to use [scan and scroll](https://www.elastic.co/guide/en/elasticsearch/guide/current/scan-scroll.html).  This is preferred to simple pagination by setting the `from` option in a search as it will keep resources open server-side allowing the next page to literally carry-on from where it was, rather than having to execute additional queries.  The downside to this is that it does require more memory/open file-handles on the server, which could go wrong if there were many un-finished scrolls; for this reason, ElasticSearch recommends a short time-out for such operations, after which it will close all resources whether the client has finished or not, the client is responsible to fetch the next page within the time-out.

To use scan and scroll, begin with a [search query](#search_query) request, but instead of calling `send` call `scan`:

```rust
let scan = client.search_query()
                 .with_indexes(&["index_name"])
                 .with_query(Query::build_match("field", "value").build())
                 .scan(Duration::new(1, DurationUnit::Minute))
                 .unwrap();
```

(Disclaimer: any use of `unwrap` in this or other example is for the purposes of brevity, obviously real code should handle errors in accordance to the needs of the application.)

Then `scroll` can be called multiple times to fetch each page.  Finally `close` will tell ElasticSearch the scan has finished and it can close any open resources.

```rust
let first_page = scan.scroll(&mut client);
// omitted - calls of subsequent pages
scan.close(&mut client).unwrap();
```

The result of the call to `scan` does not include a reference to the client, hence the need to pass in a reference to the client in subsequent calls.  The advantage of this is that that same client could be used for actions based on each `scroll`.

#### Scan and scroll with an iterator

Also supported is an iterator which will scroll through a scan.

```rust
let scan_iter = scan.iter(&mut client);
```

The iterator will include a mutable reference to the client, so the same client cannot be used concurrently.  However the iterator will automatically call `close` when it is dropped, this is so the consumer of such an iterator can use iterator functions like `take` or `take_while` without having to decide when to call `close`.

The type of each value returned from the iterator is `Result<SearchHitsHitsResult, EsError>`.  If an error is returned than it must be assumed the iterator is closed.  The type `SearchHitsHitsResult` is the same as returned in a normal search (the verbose name is intended to mirror the structure of JSON returned by ElasticSearch), as such the function [`source` is available to load the Json payload into an appropriately implemented struct](#results).

## Unimplemented features

The ElasticSearch API is made-up of a large number of smaller APIs, the vast majority of which are not yet implemented.  So far the document and search APIs are being implemented, but still to do: index management, cluster management.

A non-exhaustive (and non-prioritised) list of unimplemented APIs:

* Search Shards API (https://www.elastic.co/guide/en/elasticsearch/reference/current/search-shards.html)
* Search Templates (https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-template.html)
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

### Some, non-exhaustive, specific TODOs

1. Aggregations
2. Metric aggregations can have an empty body (check: all or some of them?) when used as a sub-aggregation underneath certain other aggregations.
2. Top-hits aggregation (will share many not-yet implemented features (e.g. highlighting): https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-metrics-top-hits-aggregation.html
2. Add significant-terms aggregation (esp., if made a permanent feature): https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-significantterms-aggregation.html
2. Add IP Range aggregation (complex due to changing response type): https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-iprange-aggregation.html
2. Reduce repetition in aggregations.rs
2. Reduce repetition around `from` functions when parsing `buckets` attribute.
2. Check for over-dependency on macros parsing from JSON
2. Consistency on when builder objects take ownership, vs. borrow a reference to some data.
2. Selective fields: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-fields.html
3. Script fields: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-script-fields.html
4. Field-data fields: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-fielddata-fields.html
5. Post filter: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-post-filter.html (after aggregations)
6. Highlighting: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-highlighting.html and Field Highlighting Order: https://www.elastic.co/guide/en/elasticsearch/reference/current/explicit-field-order.html
7. Rescoring: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-rescore.html
8. Search templates (possibly)
9. Implement Update API.
10. Implement Multi Get API
11. Implement Term Vectors and Multi termvectors API
12. Test coverage.
13. Performance (ensure use of persistent HTTP connections, etc.).
14. Replace ruby code-gen script, and replace with a Cargo build script (http://doc.crates.io/build-script.html)
15. All URI options are just String (or things that implement ToString), sometimes the values will be arrays that should be coerced into various formats.
16. Check type of "timeout" option on Search...
17. Review consistency in Operation objects (e.g. taking ownership of strings, type of parameters, etc.)
18. Index boost: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-index-boost.html
19. Shard preference: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-preference.html
20. Explain: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-preference.html
21. Add version: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-version.html
22. Inner-hits: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-inner-hits.html
23. Documentation, both rustdoc and a suitable high-level write-up in this README
24. Consider not using to_string pattern for converting to String (to avoid confusion with built-in to_string that uses formatter).
25. Avoid calls to `.to_json()` in cases where `Json::Whatever(thing)` would do instead.
26. Tidy-up/standardise logging.

## Licence

```
   Copyright 2015 Ben Ashford

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
