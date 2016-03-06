# rs-es

[![Build Status](https://travis-ci.org/benashford/rs-es.svg?branch=master)](https://travis-ci.org/benashford/rs-es)
[![](http://meritbadge.herokuapp.com/rs-es)](https://crates.io/crates/rs-es)
[![Coverage Status](https://coveralls.io/repos/github/benashford/rs-es/badge.svg?branch=master)](https://coveralls.io/github/benashford/rs-es?branch=master)

An experimental ElasticSearch client for Rust via the REST API.

Development is ongoing, and is experimental, as such breaking changes are likely at any time.  Also, large parts of the ElasticSearch API are currently unimplemented.

Versions up-to and including 0.2 of `rs-es` targetted ElasticSearch 1.6.x.  Starting with the (as yet unpublished) 0.3, the baseline has been moved up to ElasticSearch 2.0, with the intention of rapidly moving on to 2.1 and 2.2 after testing.

Please note, due a minor breaking change between Rust 1.5 and 1.6 the 0.1.x releases of `rs-es` only work with Rust 1.5 or earlier, the 0.2.x releases only work with Rust 1.6 or later.

### Contributing and compatibility

The HEAD of `master` is currently the development branch for 0.3.0, for any fixes etc. the current 0.2.x release, please open a pull request against the `0.2-releases` branch.  For contributions for ongoing development, please open a pull request against `master`.

## Documentation

[Full documentation for `rs-es`](http://benashford.github.io/rs-es/rs_es/index.html), currently accurate for the 0.2.x series.  The rest of this document consists of an introduction, describing 0.3.x.

## Building and installation

### [crates.io](http://crates.io)

Available from [crates.io](https://crates.io/crates/rs-es).

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

A search query also supports [scan and scroll](#scan-and-scroll), [sorting](#sorting), and [aggregations](#aggregations).

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
    println!("Json document: {:?}", hit.source.unwrap());
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

WARNING: In the forthcoming 0.3.0 release of `rs-es` there will be breaking changes here.  This is due to changes in ElasticSearch in the 2.0 series.  Essentially the difference between queries and filters is being removed as they will be context sensitive instead.  As such examples here might need subtle changes to work with 0.3.  E.g. `Filter::build_range("field_name")` will become `Query::build_range("field_name").

ElasticSearch offers a [rich DSL for searches](https://www.elastic.co/guide/en/elasticsearch/reference/1.x/query-dsl.html).  It is JSON based, and therefore very easy to use and composable if using from a dynamic language (e.g. [Ruby](https://github.com/elastic/elasticsearch-ruby/tree/master/elasticsearch-dsl#features-overview)); but Rust, being a staticly-typed language, things are different.  The `rs_es::query` module defines a set of builder objects which can be similarly composed to the same ends.

For example:

```rust
let query = Query::build_bool()
    .with_must(vec![Query::build_term("field_a",
                                      "value").build(),
                    Query::build_range("field_b")
                          .with_gte(5)
                          .with_lt(10)
                          .build()])
    .build();
```

The resulting `Query` value can be used in the various search/query functions exposed by [the client](#the-client).  It implements [`ToJson`](http://doc.rust-lang.org/rustc-serialize/rustc_serialize/json/index.html), which in the above example would produce JSON like so:

```javascript
{
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
```

The implementation makes much use of [conversion traits](http://benashford.github.io/blog/2015/05/24/rust-traits-for-developer-friendly-libraries/) which are used to keep a lid on the verbosity of using such a builder pattern.

### Scan and scroll

When working with large result sets that need to be loaded from an ElasticSearch query, the most efficient way is to use [scan and scroll](https://www.elastic.co/guide/en/elasticsearch/guide/current/scan-scroll.html).  This is preferred to simple pagination by setting the `from` option in a search as it will keep resources open server-side allowing the next page to literally carry-on from where it was, rather than having to execute additional queries.  The downside to this is that it does require more memory/open file-handles on the server, which could go wrong if there were many un-finished scrolls; for this reason, ElasticSearch recommends a short time-out for such operations, after which it will close all resources whether the client has finished or not, the client is responsible to fetch the next page within the time-out.

To use scan and scroll, begin with a [search query](#search_query) request, but instead of calling `send` call `scan`:

```rust
let scan = client.search_query()
                 .with_indexes(&["index_name"])
                 .with_query(Query::build_match("field", "value").build())
                 .scan(Duration::minutes(1))
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

### Aggregations

Experimental support for aggregations is also supported.

```rust
client.search_query().with_indexes(&[index_name]).with_aggs(&aggs).send();
```

Where `aggs` is a `rs_es::operations::search::aggregations::Aggregations`, for convenience sake conversion traits are implemented for common patterns; specifically the tuple `(&str, Aggregation)` for a single aggregation, and `Vec<(&str, Aggregation)>` for multiple aggregations.

Bucket aggregations (i.e. those that define a bucket that can contain sub-aggregations) can also be specified as a tuple `(Aggregation, Aggregations)`.

```rust
let aggs = Aggregations::from(("str",
                               (Terms::new("str_field").with_order(Order::asc(OrderKey::Term)),
                                Aggregations::from(("int",
                                                    Min::new("int_field"))))));

```

The above would, when used within a `search_query` operation, generate a JSON fragment within the search request:

```
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

Aggregation results are accessed in a similar way to accessing Json fields in the `rustc-serialize` library, e.g. to get the a reference to the result of the Terms aggregation called `str` (see above):

```rust
let terms_result = result.aggs_ref()
    .unwrap()
    .get("str")
    .unwrap()
    .as_terms()
    .unwrap()
```

EXPERIMENTAL: the structure of results may change as it currently feels quite cumbersome, however given this seems to be an established pattern (see the rustc-serialize project), it may not change that much.

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

0. Identify and fix duplication in SortBy(Script) and Functions Inline (Query DSL); and Scripted aggregations.
0. Implement ScriptQuery (Query DSL).
1. Implement changes to the Query API as documented here: https://www.elastic.co/guide/en/elasticsearch/reference/current/breaking_20_query_dsl_changes.html
2. Transcribe this TODO list into specific GitHub issues, for easier management.
2. Implement search API changes: https://www.elastic.co/guide/en/elasticsearch/reference/current/breaking_20_search_changes.html
3. Implement aggregation changes: https://www.elastic.co/guide/en/elasticsearch/reference/current/breaking_20_aggregation_changes.html
4. Implement scripting changes: https://www.elastic.co/guide/en/elasticsearch/reference/current/breaking_20_scripting_changes.html
5. Upgrade the targeted version of ElasticSearch from 1.6 to 2.2, paying attention to the changelogs: https://www.elastic.co/guide/en/elasticsearch/reference/current/breaking-changes-2.0.html
6. Documentation.
6. Move longer examples from README to the rustdocs instead.
6. rustc-serialize appears to be deprecated, retrofit `rs-es` to Serde: https://github.com/serde-rs/serde
6. Tests
7. Stop panicking on unexpected JSON, etc., to guard against surprises with future versions; return a result instead.
7. Metric aggregations can have an empty body (check: all or some of them?) when used as a sub-aggregation underneath certain other aggregations.
8. Top-hits aggregation (will share many not-yet implemented features (e.g. highlighting): https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-metrics-top-hits-aggregation.html
9. Add significant-terms aggregation (esp., if made a permanent feature): https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-significantterms-aggregation.html
10. Add IP Range aggregation (complex due to changing response type): https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-iprange-aggregation.html
11. Reduce repetition in aggregations.rs and/or differences with the Query DSL
12. Reduce repetition around `from` functions when parsing `buckets` attribute.
13. Check for over-dependency on macros parsing from JSON
14. Consistency on when builder objects take ownership, vs. borrow a reference to some data.
15. Selective fields: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-fields.html
16. Script fields: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-script-fields.html
17. Field-data fields: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-fielddata-fields.html
18. Post filter: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-post-filter.html (after aggregations)
19. Highlighting: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-highlighting.html and Field Highlighting Order: https://www.elastic.co/guide/en/elasticsearch/reference/current/explicit-field-order.html
20. Rescoring: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-rescore.html
21. Search templates (possibly)
22. Implement Update API.
23. Implement Multi Get API
24. Implement Term Vectors and Multi termvectors API
26. Performance (ensure use of persistent HTTP connections, etc.).
27. Replace ruby code-gen script, and replace with a Cargo build script (http://doc.crates.io/build-script.html)
28. All URI options are just String (or things that implement ToString), sometimes the values will be arrays that should be coerced into various formats.
29. Check type of "timeout" option on Search...
30. Review consistency in Operation objects (e.g. taking ownership of strings, type of parameters, etc.)
31. Index boost: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-index-boost.html
32. Shard preference: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-preference.html
33. Explain: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-preference.html
34. Add version: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-version.html
35. Inner-hits: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-inner-hits.html
36. Consider not using to_string pattern for converting to String (to avoid confusion with built-in to_string that uses formatter).
37. Avoid calls to `.to_json()` in cases where `Json::Whatever(thing)` would do instead.
38. Tidy-up/standardise logging.

## Licence

```
   Copyright 2015-2016 Ben Ashford

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
