Experimental ElasticSearch client in Rust

## Background

The only existing ElasticSearch client for Rust that I could find required the use of a ZeroMQ plugin for ElasticSearch.  This project is an ongoing implementation of an ElasticSearch client via the REST API.

## Unstable!

Currently it indexes any structure that implements `ToJson`, and nothing else.  And what it currently does may change.

## TODO

1. Full implementation of the Query DSL.
2. Implementation of Search API.
3. Test coverage.
4. Everything else.
5. Performance (ensure use of persistent HTTP connections, etc.).
6. Documentation, both rustdoc and a suitable high-level write-up in this README
