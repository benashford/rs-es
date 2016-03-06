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

use ::units::{Distance, DistanceType, GeoBox, Location};

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

    //build!(GeoShape);
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

/// Geo Bounding Box Query
#[derive(Debug, Default)]
pub struct GeoBoundingBoxQuery {
    field: String,
    geo_box: GeoBox,
    coerce: Option<bool>,
    ignore_malformed: Option<bool>,
    filter_type: Option<Type>
}

impl Query {
    pub fn build_geo_bounding_box<A, B>(field: A, geo_box: B) -> GeoBoundingBoxQuery
        where A: Into<String>,
              B: Into<GeoBox> {
        GeoBoundingBoxQuery {
            field: field.into(),
            geo_box: geo_box.into(),
            ..Default::default()
        }
    }
}

impl GeoBoundingBoxQuery {
    add_option!(with_coerce, coerce, bool);
    add_option!(with_ignore_malformed, ignore_malformed, bool);
    add_option!(with_type, filter_type, Type);

    //build!(GeoBoundingBox);
}

impl ToJson for GeoBoundingBoxQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert(self.field.clone(), self.geo_box.to_json());
        optional_add!(self, d, coerce);
        optional_add!(self, d, ignore_malformed);
        optional_add!(self, d, filter_type, "type");
        Json::Object(d)
    }
}

/// Geo Bounding Box filter type
#[derive(Debug)]
pub enum Type {
    Indexed,
    Memory
}

impl ToJson for Type {
    fn to_json(&self) -> Json {
        match self {
            &Type::Indexed => "indexed",
            &Type::Memory => "memory"
        }.to_json()
    }
}

/// Geo Distance query
///
/// TODO: Specific full unit test for querying with a generated query from here
#[derive(Debug, Default)]
pub struct GeoDistanceQuery {
    field: String,
    location: Location,
    distance: Distance,
    distance_type: Option<DistanceType>,
    optimize_bbox: Option<OptimizeBbox>,
    coerce: Option<bool>,
    ignore_malformed: Option<bool>
}

impl Query {
    pub fn build_geo_distance<A, B, C>(field: A,
                                       location: B,
                                       distance: C) -> GeoDistanceQuery
        where A: Into<String>,
              B: Into<Location>,
              C: Into<Distance> {
        GeoDistanceQuery {
            field: field.into(),
            location: location.into(),
            distance: distance.into(),
            ..Default::default()
        }
    }
}

impl GeoDistanceQuery {
    add_option!(with_distance_type, distance_type, DistanceType);
    add_option!(with_optimize_bbox, optimize_bbox, OptimizeBbox);
    add_option!(with_coerce, coerce, bool);
    add_option!(with_ignore_malformed, ignore_malformed, bool);

    //build!(GeoDistance);
}

impl ToJson for GeoDistanceQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("distance".to_owned(), self.distance.to_json());
        optional_add!(self, d, distance_type);
        optional_add!(self, d, optimize_bbox);
        d.insert(self.field.clone(), self.location.to_json());
        optional_add!(self, d, coerce);
        optional_add!(self, d, ignore_malformed);
        Json::Object(d)
    }
}

/// Options for `optimize_bbox`
#[derive(Debug)]
pub enum OptimizeBbox {
    Memory,
    Indexed,
    None
}

impl ToJson for OptimizeBbox {
    fn to_json(&self) -> Json {
        match self {
            &OptimizeBbox::Memory => "memory".to_json(),
            &OptimizeBbox::Indexed => "indexed".to_json(),
            &OptimizeBbox::None => "none".to_json()
        }
    }
}

/// Geo Polygon query
#[derive(Debug, Default)]
pub struct GeoPolygonQuery {
    field: String,
    points: Vec<Location>,
    coerce: Option<bool>,
    ignore_malformed: Option<bool>
}

impl Query {
    pub fn build_geo_polygon<A, B>(field: A,
                                   points: B) -> GeoPolygonQuery
        where A: Into<String>,
              B: Into<Vec<Location>> {
        GeoPolygonQuery {
            field: field.into(),
            points: points.into(),
            ..Default::default()
        }
    }
}

impl GeoPolygonQuery {
    add_option!(with_coerce, coerce, bool);
    add_option!(with_ignore_malformed, ignore_malformed, bool);

    //build!(GeoPolygon);
}

impl ToJson for GeoPolygonQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert("points".to_owned(), self.points.to_json());
        d.insert(self.field.clone(), Json::Object(inner));
        Json::Object(d)
    }
}

/// Geohash cell query
#[derive(Debug, Default)]
pub struct GeohashCellQuery {
    field: String,
    location: Location,
    precision: Option<Precision>,
    neighbors: Option<bool>,
}

impl Query {
    pub fn build_geohash_cell<A, B>(field: A, location: B) -> GeohashCellQuery
        where A: Into<String>,
              B: Into<Location> {
        GeohashCellQuery {
            field: field.into(),
            location: location.into(),
            ..Default::default()
        }
    }
}

impl GeohashCellQuery {
    add_option!(with_precision, precision, Precision);
    add_option!(with_neighbors, neighbors, bool);

    //build!(GeohashCell);
}

impl ToJson for GeohashCellQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert(self.field.clone(), self.location.to_json());
        optional_add!(self, d, precision);
        optional_add!(self, d, neighbors);
        Json::Object(d)
    }
}

#[derive(Debug)]
pub enum Precision {
    Geohash(u64),
    Distance(Distance)
}

impl Default for Precision {
    fn default() -> Self {
        Precision::Distance(Default::default())
    }
}

from!(u64, Precision, Geohash);
from!(Distance, Precision, Distance);

impl ToJson for Precision {
    fn to_json(&self) -> Json {
        match self {
            &Precision::Geohash(geohash_precision) => Json::U64(geohash_precision),
            &Precision::Distance(ref distance)     => distance.to_json()
        }
    }
}
