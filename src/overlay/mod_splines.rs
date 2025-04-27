use egui::{emath::RectTransform, Color32, Pos2, Shape};

use crate::{dimensions::Dimensions, views::editor_panel::EditorData};

pub fn get_segments_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    editor_data: &EditorData,
) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();

    // go through all segments
    for segment in editor_data.segments.iter().filter(|s| s.selected) {
        // get the route1 and route2 points
        let mut points = Vec::new();
        if let Some(route1) = &segment.route1 {
            for point in route1 {
                points.push([point.x, point.y]);
            }
        }

        for point in points.iter() {
            let pos2 = Pos2::new(point[0], point[1]);
            let canvas_pos = dimensions.to_canvas(pos2);

            let dot = Shape::circle_filled(to_screen * canvas_pos, 2.0, Color32::RED);
            shapes.push(dot);
        }

        // connect the points with lines
        for i in 0..points.len() - 1 {
            let p0 = Pos2::new(points[i][0], points[i][1]);
            let p1 = Pos2::new(points[i + 1][0], points[i + 1][1]);

            let line = Shape::LineSegment {
                points: [
                    to_screen * dimensions.to_canvas(p0),
                    to_screen * dimensions.to_canvas(p1),
                ],
                stroke: egui::Stroke::new(2.0, Color32::YELLOW),
            };
            shapes.push(line);
        }
    }
    shapes
}
