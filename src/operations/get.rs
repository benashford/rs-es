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

//! Implementation of the Get API

use serde::de::DeserializeOwned;

use ::{Client, EsResponse};
use ::error::EsError;
use ::util::StrJoin;
use super::common::{Options, OptionVal};

/// Values for the `preference` query parameter
pub enum Preference {
    Primary,
    Local
}

impl From<Preference> for OptionVal {
    fn from(from: Preference) -> OptionVal {
        OptionVal(match from {
            Preference::Primary => "_primary",
            Preference::Local => "_local"
        }.to_owned())
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
               id:       &'b str) -> Self {
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
        self.options.push("fields", fields.iter().join(","));
        self
    }

    add_option!(with_realtime, "realtime");
    add_option!(with_source, "_source");
    add_option!(with_routing, "routing");
    add_option!(with_preference, "preference");
    add_option!(with_refresh, "refresh");
    add_option!(with_version, "version");
    add_option!(with_version_type, "version_type");

    pub fn send<T>(&'b mut self) -> Result<GetResult<T>, EsError>
        where T: DeserializeOwned {

        let url = format!("/{}/{}/{}{}",
                          self.index,
                          self.doc_type.expect("No doc_type specified"),
                          self.id,
                          self.options);
        // We're ignoring status_code as all valid codes should return a value,
        // so anything else is an error.
        let response = self.client.get_op(&url)?;
        Ok(response.read_response()?)
    }
}

impl Client {
    /// Implementation of the ES GET API
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/docs-get.html
    pub fn get<'a>(&'a mut self,
                   index: &'a str,
                   id:    &'a str) -> GetOperation {
        GetOperation::new(self, index, id)
    }
}

/// The result of a GET request
#[derive(Debug, Deserialize)]
pub struct GetResult<T> {
    #[serde(rename="_index")]
    pub index:    String,
    #[serde(rename="_type")]
    pub doc_type: String,
    #[serde(rename="_id")]
    pub id:       String,
    #[serde(rename="_version")]
    pub version:  Option<u64>,
    pub found:    bool,
    #[serde(rename="_source")]
    pub source:   Option<T>
}

#[cfg(test)]
pub mod tests {
    use ::tests::{clean_db, TestDocument, make_client};

    #[test]
    fn test_get() {
        let index_name = "test_get";
        let mut client = make_client();
        clean_db(&mut client, index_name);
        {
            let doc = TestDocument::new().with_int_field(3)
                                         .with_bool_field(false);
            client
                .index(index_name, "test_type")
                .with_id("TEST_GETTING")
                .with_doc(&doc)
                .send().unwrap();
        }
        {
            let mut getter = client.get(index_name, "TEST_GETTING");
            let result_wrapped = getter
                .with_doc_type("test_type")
                .send();
            let result = result_wrapped.unwrap();
            assert_eq!(result.id, "TEST_GETTING");

            let source:TestDocument = result.source.expect("Source document");
            assert_eq!(source.str_field, "I am a test");
            assert_eq!(source.int_field, 3);
            assert_eq!(source.bool_field, false);
        }
    }
}
