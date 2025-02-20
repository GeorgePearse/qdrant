//! Contains functions for interpreting filter queries and defining if given points pass the conditions

use crate::types::{
    GeoBoundingBox, GeoRadius, Match, MatchValue, Range, ValueVariants, ValuesCount,
};
use serde_json::Value;

pub trait ValueChecker {
    fn check_match(&self, payload: &Value) -> bool;

    fn check(&self, payload: &Value) -> bool {
        match payload {
            Value::Array(values) => values.iter().any(|x| self.check_match(x)),
            _ => self.check_match(payload),
        }
    }
}

impl ValueChecker for Match {
    fn check_match(&self, payload: &Value) -> bool {
        match self {
            Match::Value(MatchValue { value }) => match (payload, value) {
                (Value::Bool(stored), ValueVariants::Bool(val)) => stored == val,
                (Value::String(stored), ValueVariants::Keyword(val)) => stored == val,
                (Value::Number(stored), ValueVariants::Integer(val)) => {
                    stored.as_i64().map(|num| num == *val).unwrap_or(false)
                }
                _ => false,
            },
            _ => panic!("use of deprecated conditions"),
        }
    }
}

impl ValueChecker for Range {
    fn check_match(&self, payload: &Value) -> bool {
        match payload {
            Value::Number(num) => num
                .as_f64()
                .map(|number| self.check_range(number))
                .unwrap_or(false),
            _ => false,
        }
    }
}

impl ValueChecker for GeoBoundingBox {
    fn check_match(&self, payload: &Value) -> bool {
        match payload {
            Value::Object(obj) => {
                let lon_op = obj.get("lon").and_then(|x| x.as_f64());
                let lat_op = obj.get("lat").and_then(|x| x.as_f64());

                if let (Some(lon), Some(lat)) = (lon_op, lat_op) {
                    return self.check_point(lon, lat);
                }
                false
            }
            _ => false,
        }
    }
}

impl ValueChecker for GeoRadius {
    fn check_match(&self, payload: &Value) -> bool {
        match payload {
            Value::Object(obj) => {
                let lon_op = obj.get("lon").and_then(|x| x.as_f64());
                let lat_op = obj.get("lat").and_then(|x| x.as_f64());

                if let (Some(lon), Some(lat)) = (lon_op, lat_op) {
                    return self.check_point(lon, lat);
                }
                false
            }
            _ => false,
        }
    }
}

impl ValueChecker for ValuesCount {
    fn check_match(&self, payload: &Value) -> bool {
        self.check_count(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GeoPoint;
    use serde_json::json;

    #[test]
    fn test_geo_matching() {
        let berlin_and_moscow = json!([
            {
                "lat": 52.52197645,
                "lon": 13.413637435864272
            },
            {
                "lat": 55.7536283,
                "lon": 37.62137960067377,
            }
        ]);

        let near_berlin_query = GeoRadius {
            center: GeoPoint {
                lat: 52.511,
                lon: 13.423637,
            },
            radius: 2000.0,
        };
        let miss_geo_query = GeoRadius {
            center: GeoPoint {
                lat: 52.511,
                lon: 20.423637,
            },
            radius: 2000.0,
        };

        assert!(near_berlin_query.check(&berlin_and_moscow));
        assert!(!miss_geo_query.check(&berlin_and_moscow));
    }
}
