/*
 * Copyright 2015-2016 Ben Ashford
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

//! Implementations of the Count API

use hyper::status::StatusCode;

use ::{Client, EsResponse};
use ::error::EsError;
use ::json::ShouldSkip;
use ::query::Query;
use ::operations::common::{Options, OptionVal};
use ::operations::{format_indexes_and_types, ShardCountResult};

/// Representing a count operation
#[derive(Debug)]
pub struct CountURIOperation<'a, 'b> {
    client: &'a mut Client,
    indexes: &'b [&'b str],
    doc_types: &'b [&'b str],
    options: Options<'b>
}

impl<'a, 'b> CountURIOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> CountURIOperation<'a, 'b> {
        CountURIOperation {
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

    pub fn with_query<S: Into<String>>(&'b mut self, qs: S) -> &'b mut Self {
        self.options.push("q", qs.into());
        self
    }

    add_option!(with_df, "df");
    add_option!(with_analyzer, "analyzer");
    add_option!(with_default_operator, "default_operator");
    add_option!(with_lenient, "lenient");
    add_option!(with_analyze_wildcard, "analyze_wildcard");
    add_option!(with_terminate_after, "terminate_after");

    pub fn send(&'b mut self) -> Result<CountResult, EsError> {
        let url = format!("/{}/_count{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        info!("Counting with: {}", url);
        let response = self.client.get_op(&url)?;
        match response.status_code() {
            &StatusCode::Ok => Ok(response.read_response()?),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }
}

#[derive(Default, Serialize)]
struct CountQueryOperationBody<'b> {
    /// The query
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    query: Option<&'b Query>,
}

#[derive(Debug)]
pub struct CountQueryOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes to which this query applies
    indexes: &'b [&'b str],

    /// The types to which the query applies
    doc_types: &'b [&'b str],

    /// Optionals
    options: Options<'b>,

    /// The query body
    body: CountQueryOperationBody<'b>
}

impl <'a, 'b> CountQueryOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> CountQueryOperation<'a, 'b> {
        CountQueryOperation {
            client:    client,
            indexes:   &[],
            doc_types: &[],
            options:   Options::new(),
            body:      Default::default()
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

    add_option!(with_df, "df");
    add_option!(with_analyzer, "analyzer");
    add_option!(with_default_operator, "default_operator");
    add_option!(with_lenient, "lenient");
    add_option!(with_analyze_wildcard, "analyze_wildcard");
    add_option!(with_terminate_after, "terminate_after");

    /// Performs the count with the specified query and options
    pub fn send(&'b mut self) -> Result<CountResult, EsError> {
        let url = format!("/{}/_count{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        let response = self.client.post_body_op(&url, &self.body)?;
        match response.status_code() {
            &StatusCode::Ok => Ok(response.read_response()?),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }

}

impl Client {
    /// Count via the query parameter
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-uri-request.html
    pub fn count_uri<'a>(&'a mut self) -> CountURIOperation {
        CountURIOperation::new(self)
    }

    /// Count via the query DSL
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-request-body.html
    pub fn count_query<'a>(&'a mut self) -> CountQueryOperation {
        CountQueryOperation::new(self)
    }
}

#[derive(Debug, Deserialize)]
pub struct CountResult {
    pub count: u64,

    #[serde(rename="_shards")]
    pub shards: ShardCountResult
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    extern crate regex;

    use ::tests::{clean_db, make_client};
    use ::tests::setup_test_data;

    use ::query::Query;

    use super::CountResult;

    #[test]
    fn test_count_uri() {
        let index_name = "test_count_uri";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let all_results: CountResult = client
            .count_uri()
            .with_indexes(&[index_name])
            .send()
            .unwrap();
        assert_eq!(3, all_results.count);

        let doc_1: CountResult = client
            .count_uri()
            .with_indexes(&[index_name])
            .with_query("str_field:1ABC")
            .send()
            .unwrap();
        assert_eq!(1, doc_1.count);

        let not_found_doc: CountResult = client
            .count_uri()
            .with_indexes(&[index_name])
            .with_query("str_field:lolol")
            .send()
            .unwrap();
        assert_eq!(0, not_found_doc.count);
    }

    #[test]
    fn test_count_query() {
        let index_name = "test_count_query";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let all_results: CountResult = client
            .count_query()
            .with_indexes(&[index_name])
            .with_query(&Query::build_match_all().build())
            .send()
            .unwrap();
        assert_eq!(3, all_results.count);

        let doc_1: CountResult = client
            .count_query()
            .with_indexes(&[index_name])
            .with_query(&Query::build_range("int_field")
                        .with_gte(2)
                        .with_lte(3)
                        .build())
            .send()
            .unwrap();
        assert_eq!(2, doc_1.count);

        let not_found_doc: CountResult = client
            .count_query()
            .with_indexes(&[index_name])
            .with_query(&Query::build_range("int_field")
                        .with_gte(99)
                        .build())
            .send()
            .unwrap();
        assert_eq!(0, not_found_doc.count);
    }
}
