use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl PanelFrame {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_sets_fields() {
        let frame = PanelFrame::new(10.0, 20.0, 400.0, 300.0);
        assert_eq!(frame.x, 10.0);
        assert_eq!(frame.y, 20.0);
        assert_eq!(frame.width, 400.0);
        assert_eq!(frame.height, 300.0);
    }

    #[test]
    fn serializes_with_camel_case_keys() {
        let frame = PanelFrame::new(1.0, 2.0, 3.0, 4.0);
        let json = serde_json::to_string(&frame).unwrap();
        assert_eq!(json, r#"{"x":1.0,"y":2.0,"width":3.0,"height":4.0}"#);
    }

    #[test]
    fn round_trip_through_json_preserves_values() {
        let frame = PanelFrame::new(12.5, -3.25, 512.0, 256.0);
        let json = serde_json::to_string(&frame).unwrap();
        let decoded: PanelFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, frame);
    }
}
