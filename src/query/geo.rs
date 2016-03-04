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

//! Geo queries

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

use super::Query;

/// Or
/// TODO: should probably be general purpose
#[derive(Debug)]
pub enum Or<A, B> {
    A(A),
    B(B)
}

/// GeoShape query
#[derive(Debug, Default)]
pub struct GeoShapeQuery {
    field: String,
    shape: Option<Or<Shape, IndexedShape>>
}

impl Query {
    pub fn build_geo_shape<A>(field: A) -> GeoShapeQuery
        where A: Into<String> {

        GeoShapeQuery {
            field: field.into(),
            ..Default::default()
        }
    }
}

impl GeoShapeQuery {
    pub fn with_shape<A>(mut self, shape: A) -> Self
        where A: Into<Shape> {

        self.shape = Some(Or::A(shape.into()));
        self
    }

    pub fn with_indexed_shape<A>(mut self, indexed_shape: A) -> Self
        where A: Into<IndexedShape> {

        self.shape = Some(Or::B(indexed_shape.into()));
        self
    }

    build!(GeoShape);
}

impl ToJson for GeoShapeQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();
        match self.shape {
            Some(ref o) => {
                match o {
                    &Or::A(ref shape) => {
                        inner.insert("shape".to_owned(), shape.to_json());
                    },
                    &Or::B(ref shape) => {
                        inner.insert("indexed_shape".to_owned(), shape.to_json());
                    }
                }
            },
            None => ()
        }
        d.insert(self.field.clone(), Json::Object(inner));
        Json::Object(d)
    }
}

// Required for GeoShape

#[derive(Debug)]
pub struct Shape {
    shape_type: String,
    coordinates: Vec<(f64, f64)>
}

impl Shape {
    pub fn new<A: Into<String>>(shape_type: A, coordinates: Vec<(f64, f64)>) -> Shape {
        Shape {
            shape_type:  shape_type.into(),
            coordinates: coordinates
        }
    }
}

impl ToJson for Shape {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        inner.insert("type".to_owned(), self.shape_type.to_json());

        let coordinates:Vec<Vec<f64>> = self.coordinates
            .iter()
            .map (|&(a, b)| vec![a, b])
            .collect();
        inner.insert("coordinates".to_owned(), coordinates.to_json());

        d.insert("shape".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

#[derive(Debug)]
pub struct IndexedShape {
    id:       String,
    doc_type: String,
    index:    String,
    path:     String
}

impl IndexedShape {
    pub fn new<A, B, C, D>(id: A, doc_type: B, index: C, path: D) -> IndexedShape
        where A: Into<String>,
              B: Into<String>,
              C: Into<String>,
              D: Into<String> {
        IndexedShape {
            id: id.into(),
            doc_type: doc_type.into(),
            index: index.into(),
            path: path.into()
        }
    }
}

impl ToJson for IndexedShape {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert("id".to_owned(), self.id.to_json());
        inner.insert("type".to_owned(), self.doc_type.to_json());
        inner.insert("index".to_owned(), self.index.to_json());
        inner.insert("path".to_owned(), self.path.to_json());
        d.insert("indexed_shape".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}
