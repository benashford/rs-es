// Miscellaneous code used in numerous places

use std::iter::Iterator;

macro_rules! optional_add {
    ($map:ident, $sn:expr, $field:expr) => {
        match $sn {
            Some(ref value) => { $map.insert($field.to_string(), value.to_json()); }
            _               => ()
        }
    }
}

// A custom String-join trait as the stdlib one is currently marked as unstable.
pub trait StrJoin {
    fn join(self, join: &str) -> String;
}

impl<I, S> StrJoin for I where
    S: AsRef<str>,
    I: Iterator<Item=S> {
    fn join(self, join: &str) -> String {
        let mut s = String::new();
        for f in self {
            s.push_str(f.as_ref());
            s.push_str(join);
        }
        s.pop();
        s
    }
}
