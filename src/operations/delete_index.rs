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

//! Implementation of ElasticSearch Delete Index operation

use hyper::status::StatusCode;

use ::{Client, EsResponse};
use ::error::EsError;

use super::GenericResult;

impl Client {
    /// Delete given index
    ///
    /// TODO: ensure all options are supported, replace with a `DeleteIndexOperation` to
    /// follow the pattern defined elsewhere.
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/2.x/indices-delete-index.html
    pub fn delete_index<'a>(&'a mut self, index: &'a str) -> Result<GenericResult, EsError> {
        let url = format!("/{}/", index);
        let response = self.delete_op(&url)?;

        match response.status_code() {
            &StatusCode::Ok => Ok(response.read_response()?),
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use ::tests::{clean_db, TestDocument, make_client};

    #[test]
    fn test_delete_index() {
        let index_name = "test_delete_index";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        {
            let result = client
                .index(index_name, "test_type")
                .with_doc(&TestDocument::new().with_int_field(1))
                .send();
            assert!(result.is_ok());
        }
        {
            let result = client.delete_index(index_name);
            info!("DELETE INDEX RESULT: {:?}", result);

            assert!(result.is_ok());

            let result_wrapped = result.unwrap();
            assert!(result_wrapped.acknowledged);
        }
    }
}
