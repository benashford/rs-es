/*
 * Copyright 2016 Ben Ashford
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

//! Implementation of the "mapping" operations of ElasticSearch's Indices API

use std::collections::HashMap;

use hyper::status::StatusCode;

use ::{Client, EsResponse};
use ::error::EsError;
use ::operations::format_multi;

#[derive(Serialize)]
pub struct TypeProperties;

#[derive(Serialize)]
struct PutMappingBody<'b> {
    mappings: HashMap<&'b str, TypeProperties>
}

pub struct PutMappingOperation<'a, 'b> {
    client: &'a mut Client,
    indexes: &'b [&'b str],
    body: PutMappingBody<'b>
}

impl<'a, 'b> PutMappingOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> PutMappingOperation {
        PutMappingOperation {
            client: client,
            indexes: &[],
            body: PutMappingBody {
                mappings: HashMap::new()
            }
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn send(&mut self) -> Result<PutMappingResult, EsError> {
        let url = format_multi(&self.indexes);
        let response = try!(self.client.put_body_op(&url, &self.body));
        match response.status_code() {
            &StatusCode::Ok => Ok(try!(response.read_response())),
            _ => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

impl Client {
    pub fn put_mapping<'a>(&'a mut self) -> PutMappingOperation {
        PutMappingOperation::new(self)
    }
}

/// TODO - this struct
#[derive(Deserialize)]
pub struct PutMappingResult {

}

#[cfg(test)]
mod tests {
    
}
