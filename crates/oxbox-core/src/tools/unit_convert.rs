//! Tool: Unit Converter. Factors use SI base units; temperatures include offsets.
use crate::{CoreError, CoreResult};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Unit {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    #[serde(skip)]
    factor: f64,
    #[serde(skip)]
    offset: f64,
}
macro_rules! u {
    ($id:literal,$label:literal,$cat:literal,$factor:expr) => {
        Unit {
            id: $id,
            label: $label,
            category: $cat,
            factor: $factor,
            offset: 0.0,
        }
    };
}
static UNITS: &[Unit] = &[
    u!("m", "Meter", "length", 1.0),
    u!("km", "Kilometer", "length", 1000.0),
    u!("cm", "Centimeter", "length", 0.01),
    u!("mm", "Millimeter", "length", 0.001),
    u!("in", "Inch", "length", 0.0254),
    u!("ft", "Foot", "length", 0.3048),
    u!("yd", "Yard", "length", 0.9144),
    u!("mi", "Mile", "length", 1609.344),
    u!("nmi", "Nautical mile", "length", 1852.0),
    u!("kg", "Kilogram", "mass", 1.0),
    u!("g", "Gram", "mass", 0.001),
    u!("mg", "Milligram", "mass", 0.000001),
    u!("lb", "Pound", "mass", 0.45359237),
    u!("oz", "Ounce", "mass", 0.028349523125),
    u!("t", "Metric tonne", "mass", 1000.0),
    Unit {
        id: "kelvin",
        label: "Kelvin",
        category: "temperature",
        factor: 1.0,
        offset: 0.0,
    },
    Unit {
        id: "celsius",
        label: "Celsius",
        category: "temperature",
        factor: 1.0,
        offset: 273.15,
    },
    Unit {
        id: "fahrenheit",
        label: "Fahrenheit",
        category: "temperature",
        factor: 5.0 / 9.0,
        offset: 255.3722222222222,
    },
    u!("B", "Byte", "data", 1.0),
    u!("bit", "Bit", "data", 0.125),
    u!("kB", "Kilobyte (SI)", "data", 1000.0),
    u!("MB", "Megabyte (SI)", "data", 1e6),
    u!("GB", "Gigabyte (SI)", "data", 1e9),
    u!("TB", "Terabyte (SI)", "data", 1e12),
    u!("KiB", "Kibibyte", "data", 1024.0),
    u!("MiB", "Mebibyte", "data", 1048576.0),
    u!("GiB", "Gibibyte", "data", 1073741824.0),
    u!("TiB", "Tebibyte", "data", 1099511627776.0),
    u!("s", "Second", "time", 1.0),
    u!("ms", "Millisecond", "time", 0.001),
    u!("min", "Minute", "time", 60.0),
    u!("h", "Hour", "time", 3600.0),
    u!("day", "Day", "time", 86400.0),
    u!("week", "Week", "time", 604800.0),
    u!("m/s", "Meters per second", "speed", 1.0),
    u!("km/h", "Kilometers per hour", "speed", 1.0 / 3.6),
    u!("mph", "Miles per hour", "speed", 0.44704),
    u!("kn", "Knots", "speed", 1852.0 / 3600.0),
    u!("m2", "Square meter", "area", 1.0),
    u!("km2", "Square kilometer", "area", 1e6),
    u!("ha", "Hectare", "area", 1e4),
    u!("acre", "Acre", "area", 4046.8564224),
    u!("ft2", "Square foot", "area", 0.09290304),
    u!("L", "Liter", "volume", 1.0),
    u!("mL", "Milliliter", "volume", 0.001),
    u!("m3", "Cubic meter", "volume", 1000.0),
    u!("gal-us", "US gallon", "volume", 3.785411784),
    u!("gal-uk", "Imperial gallon", "volume", 4.54609),
    u!("cup-us", "US cup", "volume", 0.2365882365),
];
pub fn units() -> &'static [Unit] {
    UNITS
}
pub fn convert(value: f64, from: &str, to: &str) -> CoreResult<f64> {
    if !value.is_finite() {
        return Err(CoreError::InvalidInput(
            "value must be a finite number".into(),
        ));
    }
    let find = |id| {
        UNITS
            .iter()
            .find(|u| u.id == id)
            .ok_or_else(|| CoreError::InvalidInput("unknown unit".into()))
    };
    let (from, to) = (find(from)?, find(to)?);
    if from.category != to.category {
        return Err(CoreError::InvalidInput(
            "units must belong to the same category".into(),
        ));
    }
    let base = value * from.factor + from.offset;
    if from.category == "temperature" && base < -1e-10 {
        return Err(CoreError::InvalidInput(
            "temperature is below absolute zero".into(),
        ));
    }
    let result = (base - to.offset) / to.factor;
    if !result.is_finite() {
        return Err(CoreError::InvalidInput(
            "conversion exceeds the numeric range".into(),
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_category_converts() {
        for (from, to, value, expected) in [
            ("km", "m", 1.0, 1000.0),
            ("kg", "g", 2.0, 2000.0),
            ("celsius", "fahrenheit", 100.0, 212.0),
            ("KiB", "B", 1.0, 1024.0),
            ("kB", "B", 1.0, 1000.0),
            ("h", "min", 1.0, 60.0),
            ("km/h", "m/s", 36.0, 10.0),
            ("ha", "m2", 1.0, 10000.0),
            ("L", "mL", 1.0, 1000.0),
        ] {
            assert!(
                (convert(value, from, to).unwrap() - expected).abs() < 1e-8,
                "{from}"
            );
        }
    }
    #[test]
    fn rejects_bad_input_and_cross_category() {
        assert!(convert(f64::NAN, "m", "km").is_err());
        assert!(convert(1.0, "m", "kg").is_err());
        assert!(convert(-1.0, "kelvin", "celsius").is_err());
        assert!(convert(1.0, "unknown", "m").is_err());
    }
    #[test]
    fn conversion_roundtrips() {
        for unit in units() {
            let value = if unit.category == "temperature" {
                300.0
            } else {
                42.25
            };
            let base = units()
                .iter()
                .find(|u| u.category == unit.category)
                .unwrap();
            let converted = convert(value, unit.id, base.id).unwrap();
            assert!((convert(converted, base.id, unit.id).unwrap() - value).abs() < 1e-7);
        }
    }
}
