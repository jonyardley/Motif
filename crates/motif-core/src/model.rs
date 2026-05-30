use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PieceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SegmentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttemptId(pub String);

/// A rectangle on a specific page in page-relative coordinates (0..page_width, 0..page_height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub page_id: PageId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_newtypes_roundtrip_serde() {
        let p = PieceId("abc".into());
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"abc\"");
        let back: PieceId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn rect_roundtrips() {
        let r = Rect { page_id: PageId("p1".into()), x: 1.0, y: 2.0, w: 30.0, h: 40.0 };
        let json = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
}
