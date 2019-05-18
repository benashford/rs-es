/*
 * Copyright 2016-2019 Ben Ashford
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

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::{
    json::{serialize_map_optional_kv, MergeSerialize, NoOuter, ShouldSkip},
    units::{Distance, DistanceType, GeoBox, Location},
};

use super::{common::FieldBasedQuery, Query};

#[derive(Debug, Serialize)]
pub enum ShapeOption {
    #[serde(rename = "shape")]
    Shape(Shape),
    #[serde(rename = "indexed_shape")]
    IndexedShape(IndexedShape),
    #[cfg(feature = "geo")]
    #[serde(rename = "shape")]
    Geojson(geojson::Geometry),
}

from!(Shape, ShapeOption, Shape);
from!(IndexedShape, ShapeOption, IndexedShape);

/// GeoShape query
#[derive(Debug, Serialize)]
pub struct GeoShapeQuery(FieldBasedQuery<Option<ShapeOption>, NoOuter>);

impl Query {
    pub fn build_geo_shape<A>(field: A) -> GeoShapeQuery
    where
        A: Into<String>,
    {
        GeoShapeQuery(FieldBasedQuery::new(field.into(), None, NoOuter))
    }
}

impl GeoShapeQuery {
    pub fn with_shape<A>(mut self, shape: A) -> Self
    where
        A: Into<Shape>,
    {
        self.0.inner = Some(ShapeOption::Shape(shape.into()));
        self
    }

    pub fn with_indexed_shape<A>(mut self, indexed_shape: A) -> Self
    where
        A: Into<IndexedShape>,
    {
        self.0.inner = Some(ShapeOption::IndexedShape(indexed_shape.into()));
        self
    }

    #[cfg(feature = "geo")]
    /// Use a geojson object as shape.
    /// Require to enable the `geo` feature.
    pub fn with_geojson<A>(mut self, shape: A) -> Self
    where
        A: Into<geojson::Geometry>,
    {
        self.0.inner = Some(ShapeOption::Geojson(shape.into()));
        self
    }

    build!(GeoShape);
}

// Required for GeoShape
#[derive(Debug, Serialize)]
pub struct Shape {
    #[serde(rename = "type")]
    shape_type: String,
    coordinates: Vec<(f64, f64)>,
}

impl Shape {
    pub fn new<A: Into<String>>(shape_type: A, coordinates: Vec<(f64, f64)>) -> Shape {
        Shape {
            shape_type: shape_type.into(),
            coordinates,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndexedShape {
    id: String,
    doc_type: String,
    index: String,
    path: String,
}

impl IndexedShape {
    pub fn new<A, B, C, D>(id: A, doc_type: B, index: C, path: D) -> IndexedShape
    where
        A: Into<String>,
        B: Into<String>,
        C: Into<String>,
        D: Into<String>,
    {
        IndexedShape {
            id: id.into(),
            doc_type: doc_type.into(),
            index: index.into(),
            path: path.into(),
        }
    }
}

/// Geo Bounding Box Query
#[derive(Debug, Serialize)]
pub struct GeoBoundingBoxQuery(FieldBasedQuery<GeoBoundingBoxQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct GeoBoundingBoxQueryInner {
    geo_box: GeoBox,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    coerce: Option<bool>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    ignore_malformed: Option<bool>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip", rename = "type")]
    filter_type: Option<Type>,
}

impl Query {
    pub fn build_geo_bounding_box<A, B>(field: A, geo_box: B) -> GeoBoundingBoxQuery
    where
        A: Into<String>,
        B: Into<GeoBox>,
    {
        GeoBoundingBoxQuery(FieldBasedQuery::new(
            field.into(),
            GeoBoundingBoxQueryInner {
                geo_box: geo_box.into(),
                ..Default::default()
            },
            NoOuter,
        ))
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
    Memory,
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::Type::*;
        match self {
            Indexed => "indexed",
            Memory => "memory",
        }
        .serialize(serializer)
    }
}

/// Geo Distance query
///
/// TODO: Specific full unit test for querying with a generated query from here
#[derive(Debug, Serialize)]
pub struct GeoDistanceQuery(FieldBasedQuery<Location, GeoDistanceQueryOuter>);

#[derive(Debug, Default)]
struct GeoDistanceQueryOuter {
    distance: Distance,
    distance_type: Option<DistanceType>,
    optimize_bbox: Option<OptimizeBbox>,
    coerce: Option<bool>,
    ignore_malformed: Option<bool>,
}

impl MergeSerialize for GeoDistanceQueryOuter {
    fn merge_serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
    where
        S: SerializeMap,
    {
        serializer.serialize_entry("distance", &self.distance)?;
        serialize_map_optional_kv(serializer, "distance_type", &self.distance_type)?;
        serialize_map_optional_kv(serializer, "optimize_bbox", &self.optimize_bbox)?;
        serialize_map_optional_kv(serializer, "coerce", &self.coerce)?;
        serialize_map_optional_kv(serializer, "ignore_malformed", &self.ignore_malformed)?;
        Ok(())
    }
}

impl Query {
    pub fn build_geo_distance<A, B, C>(field: A, location: B, distance: C) -> GeoDistanceQuery
    where
        A: Into<String>,
        B: Into<Location>,
        C: Into<Distance>,
    {
        let outer = GeoDistanceQueryOuter {
            distance: distance.into(),
            ..Default::default()
        };
        GeoDistanceQuery(FieldBasedQuery::new(field.into(), location.into(), outer))
    }
}

impl GeoDistanceQuery {
    add_outer_field!(with_distance_type, distance_type, DistanceType);
    add_outer_field!(with_optimize_bbox, optimize_bbox, OptimizeBbox);
    add_outer_field!(with_coerce, coerce, bool);
    add_outer_field!(with_ignore_malformed, ignore_malformed, bool);

    build!(GeoDistance);
}

/// Options for `optimize_bbox`
#[derive(Debug)]
pub enum OptimizeBbox {
    Memory,
    Indexed,
    None,
}

impl Serialize for OptimizeBbox {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::OptimizeBbox::*;
        match self {
            Memory => "memory".serialize(serializer),
            Indexed => "indexed".serialize(serializer),
            None => "none".serialize(serializer),
        }
    }
}

/// Geo Polygon query
#[derive(Debug, Serialize)]
pub struct GeoPolygonQuery(FieldBasedQuery<GeoPolygonQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct GeoPolygonQueryInner {
    points: Vec<Location>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    coerce: Option<bool>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    ignore_malformed: Option<bool>,
}

impl Query {
    pub fn build_geo_polygon<A, B>(field: A, points: B) -> GeoPolygonQuery
    where
        A: Into<String>,
        B: Into<Vec<Location>>,
    {
        GeoPolygonQuery(FieldBasedQuery::new(
            field.into(),
            GeoPolygonQueryInner {
                points: points.into(),
                ..Default::default()
            },
            NoOuter,
        ))
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
    fn merge_serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
    where
        S: SerializeMap,
    {
        serialize_map_optional_kv(serializer, "precision", &self.precision)?;
        serialize_map_optional_kv(serializer, "neighbors", &self.neighbors)?;
        Ok(())
    }
}

impl Query {
    pub fn build_geohash_cell<A, B>(field: A, location: B) -> GeohashCellQuery
    where
        A: Into<String>,
        B: Into<Location>,
    {
        GeohashCellQuery(FieldBasedQuery::new(
            field.into(),
            location.into(),
            Default::default(),
        ))
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
    Distance(Distance),
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
    where
        S: Serializer,
    {
        use self::Precision::*;
        match self {
            Geohash(precision) => precision.serialize(serializer),
            Distance(ref dist) => dist.serialize(serializer),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "geo")]
pub mod tests {
    use crate::operations::mapping::{Analysis, MappingOperation, Settings};
    use crate::operations::search::SearchResult;
    use crate::query::Query;
    use crate::tests::{clean_db, make_client};
    use crate::Client;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct GeoTestDocument {
        pub str_field: String,
        pub geojson_field: geojson::Geometry,
    }

    impl Default for GeoTestDocument {
        fn default() -> GeoTestDocument {
            GeoTestDocument {
                str_field: "null island".to_owned(),
                geojson_field: geojson::Geometry::new(geojson::Value::Point(vec![0.0, 0.0])),
            }
        }
    }

    impl GeoTestDocument {
        pub fn with_str_field(mut self, s: &str) -> GeoTestDocument {
            self.str_field = s.to_owned();
            self
        }

        pub fn with_point(mut self, p: Vec<f64>) -> GeoTestDocument {
            self.geojson_field = geojson::Geometry::new(geojson::Value::Point(p));
            self
        }
    }

    pub fn setup_test_data(mut client: &mut Client, index_name: &str) {
        let mut mapping = HashMap::new();
        let mut doc = HashMap::new();
        let mut geo_field = HashMap::new();
        let mut str_field = HashMap::new();
        str_field.insert("type", "string");
        geo_field.insert("type", "geo_shape");
        doc.insert("str_field", str_field);
        doc.insert("geojson_field", geo_field);
        mapping.insert("geo_test_type", doc);

        let settings = Settings {
            number_of_shards: 1,
            analysis: Analysis {
                filter: serde_json::json!({}).as_object().unwrap().clone(),
                analyzer: serde_json::json!({}).as_object().unwrap().clone(),
            },
        };

        // TODO - this fails in many cases (specifically on TravisCI), but we ignore the
        // failures anyway
        let _ = client.delete_index(index_name);

        let result = MappingOperation::new(&mut client, index_name)
            .with_mapping(&mapping)
            .with_settings(&settings)
            .send();
        result.unwrap();
        let documents = vec![
            GeoTestDocument::default(),
            GeoTestDocument::default()
                .with_str_field("p1")
                .with_point(vec![1.0, 1.0]),
            GeoTestDocument::default()
                .with_str_field("p2")
                .with_point(vec![5.0, 1.0]),
        ];
        for doc in documents.iter() {
            client
                .index(index_name, "geo_test_type")
                .with_doc(doc)
                .send()
                .unwrap();
        }
        client.refresh().with_indexes(&[index_name]).send().unwrap();
    }

    #[test]
    fn test_geoshape_search_point() {
        let index_name = "test_geoshape_search_point";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let all_results: SearchResult<GeoTestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_query(
                &Query::build_geo_shape("geojson_field")
                    .with_geojson(geojson::Geometry::new(geojson::Value::Point(vec![
                        0.0, 0.0,
                    ])))
                    .build(),
            )
            .send()
            .unwrap();
        assert_eq!(1, all_results.hits.total);
    }

    #[test]
    fn test_geoshape_search_polygon() {
        let index_name = "test_geoshape_search_polygon";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let all_results: SearchResult<GeoTestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_query(
                &Query::build_geo_shape("geojson_field")
                    .with_geojson(geojson::Geometry::new(geojson::Value::Polygon(vec![vec![
                        vec![1.0, 1.0],
                        vec![1.0, -1.0],
                        vec![-1.0, -1.0],
                        vec![-1.0, 1.0],
                        vec![1.0, 1.0],
                    ]])))
                    .build(),
            )
            .send()
            .unwrap();
        assert_eq!(2, all_results.hits.total);
    }
}
