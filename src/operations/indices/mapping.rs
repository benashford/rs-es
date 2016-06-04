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
use ::operations::{format_multi, GenericResult};

#[derive(Serialize)]
pub struct Field<'b> {
    #[serde(rename="type")]
    field_type: &'b str
}

impl<'b> From<&'b str> for Field<'b> {
    fn from(from: &'b str) -> Field<'b> {
        Field {
            field_type: from
        }
    }
}

#[derive(Serialize)]
pub struct TypeProperties<'b> {
    properties: HashMap<&'b str, Field<'b>>,
}

impl<'b> From<HashMap<&'b str, Field<'b>>> for TypeProperties<'b> {
    fn from(from: HashMap<&'b str, Field<'b>>) -> TypeProperties<'b> {
        TypeProperties {
            properties: from
        }
    }
}

impl<'b> From<(&'b str, Field<'b>)> for TypeProperties<'b> {
    fn from(from: (&'b str, Field<'b>)) -> TypeProperties<'b> {
        let mut map = HashMap::new();
        map.insert(from.0, from.1);

        map.into()
    }
}

impl<'b> From<(&'b str, &'b str)> for TypeProperties<'b> {
    fn from(from: (&'b str, &'b str)) -> TypeProperties<'b> {
        let field:Field<'b> = from.1.into();
        (from.0, field).into()
    }
}

pub type Mappings<'b> = HashMap<&'b str, TypeProperties<'b>>;

#[derive(Serialize)]
struct PutMappingBody<'b> {
    mappings: Mappings<'b>
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

    pub fn with_mappings(&'b mut self, mappings: Mappings<'b>) -> &'b mut Self {
        self.body.mappings = mappings;
        self
    }

    pub fn add_mapping<P>(&'b mut self,
                          doc_type:   &'b str,
                          properties: P) -> &'b mut Self
        where P: Into<TypeProperties<'b>> {

        self.body.mappings.insert(doc_type, properties.into());
        self
    }

    pub fn send(&mut self) -> Result<GenericResult, EsError> {
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

#[cfg(test)]
mod tests {
    use ::tests::{delete_index, make_client};

    #[test]
    fn test_put_mapping() {
        let index_name = "test_put_mappings";
        let mut client = make_client();
        delete_index(&mut client, index_name);

        let result = client.put_mapping()
            .with_indexes(&[index_name])
            .add_mapping("type", ("field_a", "string"))
            .send();
        assert!(result.unwrap().acknowledged);
    }
}
