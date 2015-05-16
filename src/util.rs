macro_rules! optional_add {
    ($map:ident, $sn:expr, $field:expr) => {
        match $sn {
            Some(ref value) => { $map.insert($field.to_string(), value.to_json()); }
            _               => ()
        }
    }
}
