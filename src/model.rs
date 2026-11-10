//! Annotation model shared by the overlay UI and the exporter.

use crate::shapes::{self, RectF, V2};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pen,
    Arrow,
    Rect,
    Highlight,
    Redact,
    Text,
}

impl Tool {
    pub const ALL: [Tool; 7] = [
        Tool::Select,
        Tool::Pen,
        Tool::Arrow,
        Tool::Rect,
        Tool::Highlight,
        Tool::Redact,
        Tool::Text,
    ];

    pub fn glyph(self) -> &'static str {
        match self {
            Tool::Select => "◻",
            Tool::Pen => "✎",
            Tool::Arrow => "↗",
            Tool::Rect => "▭",
            Tool::Highlight => "▬",
            Tool::Redact => "▩",
            Tool::Text => "T",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Pen => "Pen",
            Tool::Arrow => "Arrow",
            Tool::Rect => "Rectangle",
            Tool::Highlight => "Highlight",
            Tool::Redact => "Redact",
            Tool::Text => "Text",
        }
    }
}

/// Annotation colours are plain RGBA so the exporter can use them directly.
pub const PALETTE: [[u8; 4]; 6] = [
    [239, 68, 68, 255],  // red
    [245, 158, 11, 255], // amber
    [34, 197, 94, 255],  // green
    [59, 130, 246, 255], // blue
    [255, 255, 255, 255], // white
    [17, 24, 39, 255],   // near black
];

#[derive(Clone, Debug)]
pub enum Annotation {
    Pen {
        points: Vec<V2>,
        color: [u8; 4],
        width: f32,
    },
    Arrow {
        start: V2,
        end: V2,
        color: [u8; 4],
        width: f32,
    },
    Rect {
        rect: RectF,
        color: [u8; 4],
        width: f32,
    },
    Highlight {
        rect: RectF,
        color: [u8; 4],
    },
    Redact {
        rect: RectF,
    },
    Text {
        origin: V2,
        text: String,
        color: [u8; 4],
        size: f32,
    },
}

pub const TEXT_SIZE: f32 = 22.0;

impl Annotation {
    /// Start a new annotation of the given tool at `start`.
    pub fn new(tool: Tool, start: V2, color: [u8; 4], width: f32) -> Option<Annotation> {
        let annotation = match tool {
            Tool::Select | Tool::Text => return None,
            Tool::Pen => Annotation::Pen {
                points: vec![start],
                color,
                width,
            },
            Tool::Arrow => Annotation::Arrow {
                start,
                end: start,
                color,
                width,
            },
            Tool::Rect => Annotation::Rect {
                rect: RectF::new(start.x, start.y, 0.0, 0.0),
                color,
                width,
            },
            Tool::Highlight => Annotation::Highlight {
                rect: RectF::new(start.x, start.y, 0.0, 0.0),
                color,
            },
            Tool::Redact => Annotation::Redact {
                rect: RectF::new(start.x, start.y, 0.0, 0.0),
            },
        };
        Some(annotation)
    }

    /// Extend an in-progress annotation to `point`.
    pub fn drag_to(&mut self, point: V2) {
        match self {
            Annotation::Pen { points, .. } => {
                if points.last().map(|p| p.distance_to(point)).unwrap_or(f32::MAX) > 1.0 {
                    points.push(point);
                }
            }
            Annotation::Arrow { end, .. } => *end = point,
            Annotation::Rect { rect, .. }
            | Annotation::Highlight { rect, .. }
            | Annotation::Redact { rect } => {
                let start = V2::new(rect.x, rect.y);
                *rect = RectF::from_points(start, point);
            }
            Annotation::Text { .. } => {}
        }
    }

    pub fn is_meaningful(&self) -> bool {
        match self {
            Annotation::Pen { points, .. } => points.len() >= 2,
            Annotation::Arrow { start, end, .. } => start.distance_to(*end) > 3.0,
            Annotation::Rect { rect, .. }
            | Annotation::Highlight { rect, .. }
            | Annotation::Redact { rect } => !rect.is_empty() && rect.w > 2.0 && rect.h > 2.0,
            Annotation::Text { text, .. } => !text.trim().is_empty(),
        }
    }

    /// Filled polygons (geometry + colour) representing this annotation.
    /// Text is not included: it is rasterised separately.
    pub fn polygons(&self) -> Vec<(Vec<V2>, [u8; 4])> {
        match self {
            Annotation::Pen {
                points,
                color,
                width,
            } => shapes::polyline(points, *width)
                .into_iter()
                .map(|polygon| (polygon, *color))
                .collect(),
            Annotation::Arrow {
                start,
                end,
                color,
                width,
            } => shapes::arrow(*start, *end, *width)
                .into_iter()
                .map(|polygon| (polygon, *color))
                .collect(),
            Annotation::Rect { rect, color, width } => shapes::rect_outline(*rect, *width)
                .into_iter()
                .map(|polygon| (polygon, *color))
                .collect(),
            Annotation::Highlight { rect, color } => {
                let mut fill = *color;
                fill[3] = 0x66;
                vec![(rect.polygon(), fill)]
            }
            Annotation::Redact { rect } => vec![(rect.polygon(), [0, 0, 0, 255])],
            Annotation::Text { .. } => Vec::new(),
        }
    }

    pub fn text_parts(&self) -> Option<(V2, &str, [u8; 4], f32)> {
        match self {
            Annotation::Text {
                origin,
                text,
                color,
                size,
            } => Some((*origin, text.as_str(), *color, *size)),
            _ => None,
        }
    }
}
