use egui::{emath::RectTransform, Color32, Pos2, Shape};

use crate::{dimensions::Dimensions, views::editor_panel::EditorData};

pub fn get_segments_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    zoom: f32,
    editor_data: &EditorData,
    hover_pos: &Option<Pos2>,
) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();

    // go through all segments
    for (_e, segment) in editor_data.segments.iter().filter(|(_, s)| s.selected) {
        // is current segment?
        let is_current = if let Some(current_segment) = &editor_data.current_segment {
            current_segment == &segment.id
        } else {
            false
        };

        let mut line_color = Color32::YELLOW;
        if is_current {
            line_color = Color32::GREEN;
        }

        let mut point_color = Color32::RED;
        if is_current {
            point_color = Color32::DARK_RED;
        }

        let mut points = Vec::new();
        if let Some(route1) = &segment.route1 {
            for point in route1 {
                points.push([point.x, point.y]);
            }
        }

        // TODO get route2 points

        for point in points.iter() {
            let pos2 = Pos2::new(point[0], point[1]);
            let canvas_pos = dimensions.engine_to_canvas(pos2);
            let center = to_screen * canvas_pos;
            let mut radius = 2.0;
            if let Some(hover_pos) = hover_pos {
                if (center - *hover_pos).length() < 10.0 {
                    radius = 3.0;
                }
            }
            let dot = Shape::circle_filled(center, radius * zoom, point_color);
            shapes.push(dot);
        }

        // connect the points with lines
        for i in 0..points.len() - 1 {
            let p0 = Pos2::new(points[i][0], points[i][1]);
            let p1 = Pos2::new(points[i + 1][0], points[i + 1][1]);

            let line = Shape::LineSegment {
                points: [
                    to_screen * dimensions.engine_to_canvas(p0),
                    to_screen * dimensions.engine_to_canvas(p1),
                ],
                stroke: egui::Stroke::new(2.0, line_color),
            };
            shapes.push(line);
        }
    }

    // ports
    for (_name, port) in editor_data.ports.iter() {
        for data in port.data.values() {
            let pos2 = Pos2::new(data.position.x, data.position.y);
            let canvas_pos = dimensions.engine_to_canvas(pos2);
            let center = to_screen * canvas_pos;
            let radius = 2.0;

            let dot = Shape::circle_filled(center, radius * zoom, Color32::BLUE);
            shapes.push(dot);
        }
    }

    shapes
}
