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

use serde::ser::{Serialize, Serializer, SerializeMap};

use ::json::{MergeSerialize, NoOuter, ShouldSkip};
use ::units::{Distance, DistanceType, GeoBox, Location};

use super::Query;
use super::common::FieldBasedQuery;

#[derive(Debug, Serialize)]
pub enum ShapeOption {
    #[serde(rename="shape")]
    Shape(Shape),
    #[serde(rename="indexed_shape")]
    IndexedShape(IndexedShape)
}

from!(Shape, ShapeOption, Shape);
from!(IndexedShape, ShapeOption, IndexedShape);

/// GeoShape query
#[derive(Debug, Serialize)]
pub struct GeoShapeQuery(FieldBasedQuery<Option<ShapeOption>, NoOuter>);

impl Query {
    pub fn build_geo_shape<A>(field: A) -> GeoShapeQuery
        where A: Into<String> {

        GeoShapeQuery(FieldBasedQuery::new(field.into(), None, NoOuter))
    }
}

impl GeoShapeQuery {
    pub fn with_shape<A>(mut self, shape: A) -> Self
        where A: Into<Shape> {

        self.0.inner = Some(ShapeOption::Shape(shape.into()));
        self
    }

    pub fn with_indexed_shape<A>(mut self, indexed_shape: A) -> Self
        where A: Into<IndexedShape> {

        self.0.inner = Some(ShapeOption::IndexedShape(indexed_shape.into()));
        self
    }

    build!(GeoShape);
}

// Required for GeoShape
#[derive(Debug, Serialize)]
pub struct Shape {
    #[serde(rename="type")]
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

#[derive(Debug, Serialize)]
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

/// Geo Bounding Box Query
#[derive(Debug, Serialize)]
pub struct GeoBoundingBoxQuery(FieldBasedQuery<GeoBoundingBoxQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct GeoBoundingBoxQueryInner {
    geo_box: GeoBox,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    coerce: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    ignore_malformed: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip", rename="type")]
    filter_type: Option<Type>
}

impl Query {
    pub fn build_geo_bounding_box<A, B>(field: A, geo_box: B) -> GeoBoundingBoxQuery
        where A: Into<String>,
              B: Into<GeoBox> {
        GeoBoundingBoxQuery(FieldBasedQuery::new(field.into(),
                                                 GeoBoundingBoxQueryInner {
                                                     geo_box: geo_box.into(),
                                                     ..Default::default()
                                                 },
                                                 NoOuter))
    }
}

impl GeoBoundingBoxQuery {
    add_inner_field!(with_coerce, coerce, bool);
    add_inner_field!(with_ignore_malformed, ignore_malformed, bool);
    add_inner_field!(with_type, filter_type, Type);

    build!(GeoBoundingBox);
}

/// Geo Bounding Box filter type
#[derive(Debug)]
pub enum Type {
    Indexed,
    Memory
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::Type::*;
        match self {
            &Indexed => "indexed",
            &Memory => "memory"
        }.serialize(serializer)
    }
}

/// Geo Distance query
///
/// TODO: Specific full unit test for querying with a generated query from here
#[derive(Debug, Default, Serialize)]
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
    add_field!(with_distance_type, distance_type, DistanceType);
    add_field!(with_optimize_bbox, optimize_bbox, OptimizeBbox);
    add_field!(with_coerce, coerce, bool);
    add_field!(with_ignore_malformed, ignore_malformed, bool);

    build!(GeoDistance);
}

/// Options for `optimize_bbox`
#[derive(Debug)]
pub enum OptimizeBbox {
    Memory,
    Indexed,
    None
}

impl Serialize for OptimizeBbox {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::OptimizeBbox::*;
        match self {
            &Memory => "memory".serialize(serializer),
            &Indexed => "indexed".serialize(serializer),
            &None => "none".serialize(serializer)
        }
    }
}

/// Geo Polygon query
#[derive(Debug, Serialize)]
pub struct GeoPolygonQuery(FieldBasedQuery<GeoPolygonQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct GeoPolygonQueryInner {
    points: Vec<Location>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    coerce: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    ignore_malformed: Option<bool>
}

impl Query {
    pub fn build_geo_polygon<A, B>(field: A,
                                   points: B) -> GeoPolygonQuery
        where A: Into<String>,
              B: Into<Vec<Location>> {
        GeoPolygonQuery(FieldBasedQuery::new(field.into(),
                                             GeoPolygonQueryInner {
                                                 points: points.into(),
                                                 ..Default::default()
                                             },
                                             NoOuter))
    }
}

impl GeoPolygonQuery {
    add_inner_field!(with_coerce, coerce, bool);
    add_inner_field!(with_ignore_malformed, ignore_malformed, bool);

    build!(GeoPolygon);
}

/// Geohash cell query
#[derive(Debug, Serialize)]
pub struct GeohashCellQuery(FieldBasedQuery<Location, GeohashCellQueryOuter>);

#[derive(Debug, Default)]
pub struct GeohashCellQueryOuter {
    precision: Option<Precision>,
    neighbors: Option<bool>,
}

impl MergeSerialize for GeohashCellQueryOuter {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {

        match self.precision {
            Some(ref p) => {
                serializer.serialize_entry("precision", p)?;
            },
            None => ()
        };
        match self.neighbors {
            Some(b) => {
                serializer.serialize_entry("neighbors", &b)?;
            },
            None => ()
        }
        Ok(())
    }
}

impl Query {
    pub fn build_geohash_cell<A, B>(field: A, location: B) -> GeohashCellQuery
        where A: Into<String>,
              B: Into<Location> {
        GeohashCellQuery(FieldBasedQuery::new(field.into(),
                                              location.into(),
                                              Default::default()))
    }
}

impl GeohashCellQuery {
    add_outer_field!(with_precision, precision, Precision);
    add_outer_field!(with_neighbors, neighbors, bool);

    build!(GeohashCell);
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

impl Serialize for Precision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::Precision::*;
        match self {
            &Geohash(precision) => precision.serialize(serializer),
            &Distance(ref dist) => dist.serialize(serializer)
        }
    }
}
