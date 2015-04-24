// Query DSL

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

pub enum Query {
    MatchAll
}

use self::Query::MatchAll;

impl ToJson for Query {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::<String, Json>::new();
        match self {
            &MatchAll => { d.insert("match_all".to_string(), Json::Object(BTreeMap::new())); }
        }
        Json::Object(d)
    }
}
