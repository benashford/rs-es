Experimental ElasticSearch client in Rust

## Background

The only existing ElasticSearch client for Rust that I could find required the use of a ZeroMQ plugin for ElasticSearch.  This project is an ongoing implementation of an ElasticSearch client via the REST API.

## Unstable!

Currently it indexes any structure that implements `ToJson`, and nothing else.  And what it currently does may change.
