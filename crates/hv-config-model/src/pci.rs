//! PCI BDF parsing helpers.

use crate::error::{ConfigError, ConfigErrorKind};
use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

/// Parses a PCI BDF string in `SSSS:BB:DD.F` or `BB:DD.F` form.
pub fn parse_bdf(input: &str) -> Result<PciBdf, ConfigError> {
    let trimmed = input.trim();
    let (segment, rest) = if let Some((seg, rest)) = trimmed.split_once(':') {
        if rest.contains(':') {
            let segment = parse_u16(seg, "segment")?;
            (segment, rest)
        } else {
            (0, trimmed)
        }
    } else {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("invalid PCI BDF '{input}'"),
        ));
    };

    let parts: Vec<&str> = rest.split(':').collect();
    let [bus_part, dev_fn_part] = match parts.as_slice() {
        [bus, dev_fn] => [bus, dev_fn],
        _ => {
            return Err(ConfigError::new(
                ConfigErrorKind::Syntax,
                format!("invalid PCI BDF '{input}'"),
            ));
        }
    };

    let bus = parse_u8(bus_part, "bus")?;
    let dev_fn = dev_fn_part;
    let (device, function) = if let Some((dev, func)) = dev_fn.split_once('.') {
        (parse_u8(dev, "device")?, parse_u8(func, "function")?)
    } else {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("invalid PCI device.function in '{input}'"),
        ));
    };

    Ok(PciBdf::new(
        PciSegment::new(segment),
        PciBus::new(bus),
        PciDevice::new(device),
        PciFunction::new(function),
    ))
}

fn parse_u8(value: &str, field: &str) -> Result<u8, ConfigError> {
    if value.len() > 2 {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("{field} value '{value}' out of range"),
        ));
    }
    u8::from_str_radix(value, 16).map_err(|_| {
        ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("invalid hex {field} '{value}'"),
        )
    })
}

fn parse_u16(value: &str, field: &str) -> Result<u16, ConfigError> {
    if value.len() > 4 {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("{field} value '{value}' out of range"),
        ));
    }
    u16::from_str_radix(value, 16).map_err(|_| {
        ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("invalid hex {field} '{value}'"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_short_form() {
        let bdf = parse_bdf("00:03.0").expect("parse");
        assert_eq!(bdf.bus.raw(), 0);
        assert_eq!(bdf.device.raw(), 3);
    }

    #[test]
    fn parse_rejects_malformed_strings() {
        assert!(parse_bdf("not-a-bdf").is_err());
        assert!(parse_bdf("00:03").is_err());
        assert!(parse_bdf("00:03.").is_err());
        assert!(parse_bdf("00:.0").is_err());
        assert!(parse_bdf(":00:03.0").is_err());
    }

    #[test]
    fn parse_invalid_segment_and_bus_values() {
        assert!(parse_bdf("gg:03.0").is_err());
        assert!(parse_bdf("0000:gg:03.0").is_err());
        assert!(parse_bdf("00000:00:03.0").is_err());
        assert!(parse_bdf("0000:00:03:04.0").is_err());
        assert!(parse_bdf("00:100.0").is_err());
        assert!(parse_bdf("00:03.zz").is_err());
        assert!(parse_bdf("00:3.100").is_err());
    }

    #[test]
    fn parse_trims_whitespace() {
        let bdf = parse_bdf("  00:03.0 ").expect("parse");
        assert_eq!(bdf.device.raw(), 3);
    }

    #[test]
    fn parse_long_segment_form() {
        let bdf = parse_bdf("0001:00:03.0").expect("parse");
        assert_eq!(bdf.segment.raw(), 1);
    }
}
