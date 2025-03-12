/// ## Provides functionality for drawing various shapes for use as highlight indicator in the `pie menu`.
///
/// The `HighlightPainter` trait extends the `egui::Painter` with methods for drawing arcs, slices and circles, and combinations of these shapes.
/// The `ArcValues`, `SliceValues`, and `CircleValues` structs encapsulate the necessary parameters for drawing the corresponding shapes.
/// 
/// The `highlight_shape` method of the `HighlightPainter` trait is the main entry point for drawing the highlighted shapes, taking the desired `PieMenuHighlightShape` and the corresponding parameter structs.
use egui::{Pos2, vec2, epaint::Shape, Color32, Stroke};
use crate::settings::PieMenuHighlightShape;

/// ### Holds the necessary parameters for drawing an arc shape.
///
/// The `angle_range` field specifies the start and end angles of the arc in radians.
/// The `center` field holds the center position of the arc.
/// The `radius` field specifies the radius of the arc.
/// The `resolution` field controls the number of points generated along the arc, with a higher resolution resulting in a smoother arc.
/// The `stroke` field holds the stroke settings for the arc.
pub struct ArcValues {
    pub angle_range: std::ops::Range<f32>,
    pub center: Pos2,
    pub radius: f32,
    pub resolution: f32,
    pub stroke: egui::Stroke,
}

/// ### Holds the necessary parameters for drawing a slice shape.
///
/// The `arc_values` field contains the parameters for drawing the arc that forms the outline of the slice.
/// - Optional if shape is [PieMenuHighlightShape::ArcSlice] or [PieMenuHighlightShape::ArcSliceCircle]
/// - Required if shape is [PieMenuHighlightShape::Slice] or [PieMenuHighlightShape::SliceCircle]
/// 
/// The `stroke` field holds the stroke settings for the slice outline. `[optional]`
/// 
/// The `fill_color` field specifies the color to use for filling the slice.
pub struct SliceValues {
    pub arc_values: Option<ArcValues>, 
    pub stroke: Option<egui::Stroke>,
    pub fill_color: egui::Color32,
}

/// ### Holds the necessary parameters for drawing a circle shape.
/// - The `offset_angle` field specifies the angle in radians to offset the circle's center from the `offset_center`.
///     - Either the direction of the mouse pointer or highlighted button direction
/// - The `offset_radius` field specifies the distance to offset the circle's center from the `offset_center`.
///     - Usually the same as the `radius` for the `arc` or `slice` shapes
/// - The `offset_center` field holds the offset center position of the circle.
///     - Usually the same as the `center` for the `arc` or `slice` shapes
/// - The `circle_radius` field specifies the radius of the circle.
/// - The `stroke` field holds the stroke settings for the circle outline. `[optional]`
/// - The `fill_color` field specifies the color to use for filling the circle. `[optional]`
pub struct CircleValues {
    pub offset_angle: f32,
    pub offset_radius: f32,
    pub offset_center: Pos2,
    pub circle_radius: f32,
    pub stroke: Stroke,
    pub fill_color: Color32,
}

/// ### Extension trait for egui::Painter
/// A trait that provides methods for painting various highlighting shapes for `pie menu`.
///
/// The `highlight_shape` method is used to paint a specific shape, such as an arc, slice, or circle, based on the provided parameters.
///
/// The `arc_calculate_points` method calculates the points along an arc with the given parameters, such as the center, radius, resolution, and angle range.
///
/// The `draw_arc` method draws an arc using the provided `ArcValues` struct.
///
/// The `draw_circle` method draws a circle using the provided `CircleValues` struct.
pub trait HighlightPainter {
    fn highlight_shape(&self, shape: PieMenuHighlightShape, arc_values: Option<ArcValues>, slice_values: Option<SliceValues>, circle_values: Option<CircleValues>,);
    fn arc_calculate_points(&self, center: Pos2, radius: f32, resoltion: f32, angle_range: std::ops::Range<f32>) -> Vec<Pos2>;
    fn draw_arc(&self, arc_values: ArcValues);
    fn draw_circle(&self, circle_values: CircleValues);
}

impl HighlightPainter for egui::Painter {

    /// ### Calculates the points along an arc with the given parameters.
    ///
    /// This function takes the center position, radius, resolution, and angle range of an arc,
    /// and returns a vector of `Pos2` points representing the arc.
    ///
    /// The `angle_range` parameter specifies the start and end angles of the arc in radians.
    /// The `resolution` parameter controls the number of points generated along the arc, with
    /// a higher resolution resulting in a smoother arc.
    ///
    /// The function performs some input validation to ensure the parameters are within valid ranges.
    fn arc_calculate_points(
        &self,
        center: Pos2,
        radius: f32,
        resolution: f32,
        angle_range: std::ops::Range<f32>,) -> Vec<Pos2> {
        //assert!(angle_range.start >= 0.0 && angle_range.end <= std::f32::consts::TAU, "Angle range must be within the range of 0 to 2π radians");
        assert!(resolution > 0.0 && resolution <= 100.0, "Resolution must be between 0.0 and 100.0");
        assert!(radius > 0.0, "Radius must be greater than 0.0");
        let width_angle = angle_range.end - angle_range.start;
        let arc_length = width_angle * radius;
        let n_points = (arc_length * resolution).ceil() as usize;
        let step = width_angle / n_points as f32;
        let points: Vec<Pos2> = (0..=n_points)
            .map(|i| {
                let angle = angle_range.start + i as f32 * step;
                center + vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        points
    }



    /// ### Draws a circle on the painter using the provided `CircleValues` struct.
    ///
    /// The `circle_values` parameter contains information about the circle to be drawn, including its `center`, `radius`, `fill_color`, and `stroke`.
    ///
    /// - This function first performs assertions to ensure that the circle radius is greater than 0.0, and that either the fill color or stroke is provided. \
    /// - If the stroke and fill colors are both [egui::Color32::TRANSPARENT], the function returns without drawing anything and prints a message to the console. 
    /// - Otherwise a circle is drawn, using the provided center, radius, and colors.
    fn draw_circle(&self, circle_values: CircleValues) {
        let circle = circle_values;
        let center = circle.offset_center + circle.offset_radius * egui::vec2(circle.offset_angle.cos(), circle.offset_angle.sin());
        assert!(circle.circle_radius > 0.0, "Highlight circles `circle_radius` must be greater than 0.0");
        
        if (circle.stroke.color == egui::Color32::TRANSPARENT) && circle.fill_color == egui::Color32::TRANSPARENT {
            println!("Fill and stroke color are both TRANSPARENT, won't draw circle");
            return;
        }
        
        self.add(egui::epaint::CircleShape{
            center: center, 
            radius: circle.circle_radius, 
            fill: circle.fill_color,
            stroke: circle.stroke,
        }
    );
    }
                    
    /// ### Draws an arc on the painter using the provided `ArcValues` struct.
    ///
    /// The `arc_values` parameter contains information about the arc to be drawn, including its center, radius, resolution, angle range, and stroke.
    ///
    /// This function first performs an assertion to ensure that the stroke is not set to `egui::Stroke::NONE`. \
    /// If the stroke color is transparent, the function returns without drawing anything and prints a message to the console.\
    /// It then calculates the points along the arc using the `arc_calculate_points` function, and adds a line shape to the painter using those points and the provided stroke.
    fn draw_arc(&self, arc_values: ArcValues) {
        assert!(arc_values.stroke != egui::Stroke::NONE, "Stroke must not be NONE");
        if arc_values.stroke.color == egui::Color32::TRANSPARENT {
            println!("Stroke color is transparent, won't draw arc");
            return;
        }
        let arc = arc_values;
        let points = self.arc_calculate_points(arc.center, arc.radius, arc.resolution, arc.angle_range);
        self.add(Shape::line(points, arc.stroke));
    }
    
    /// ### Draws various shapes on the painter based on the provided `PieMenuHighlightShape` and associated values.
    ///
    /// This function handles the drawing of different shapes, including arcs, slices, circles, and combinations of these shapes. It uses the `draw_arc` and `draw_circle` functions to render the shapes on the painter.
    ///
    /// The `shape` parameter specifies the type of shape to be drawn, and the corresponding `arc_values`, `slice_values`, and `circle_values` parameters provide the necessary information to draw the shape.
    ///
    /// The function performs various assertions to ensure that the provided values are valid, such as checking the angle range, resolution, and radius.
    fn highlight_shape(&self, 
        shape: PieMenuHighlightShape,
        arc_values: Option<ArcValues>,
        slice_values: Option<SliceValues>,
        circle_values:Option<CircleValues>,) {        
        match shape {
            PieMenuHighlightShape::Arc => {
                self.draw_arc(arc_values.unwrap());
            },

            PieMenuHighlightShape::Slice => {
                let slice = slice_values.unwrap();
                assert!(slice.arc_values.is_some(), "ArcValues must be provided for slice highlight");
                let arc = slice.arc_values.unwrap();
                let mut points = self.arc_calculate_points(arc.center, arc.radius, arc.resolution, arc.angle_range);
                points.insert(0, arc.center);
                points.push(arc.center);
                self.add(Shape::convex_polygon(points, slice.fill_color, if slice.stroke.is_some() { slice.stroke.unwrap() } else { egui::Stroke::NONE }));            
            },
            
            PieMenuHighlightShape::Circle => {
                self.draw_circle( circle_values.unwrap());
            },

            PieMenuHighlightShape::ArcSlice => {
                let arc = arc_values.unwrap();
                let slice = slice_values.unwrap();
                let points = self.arc_calculate_points(arc.center, arc.radius, arc.resolution, arc.angle_range);
                let mut slice_points = points.clone();
                slice_points.insert(0, arc.center);
                slice_points.push(arc.center);
                self.add(Shape::convex_polygon(slice_points, slice.fill_color, if slice.stroke.is_some() { slice.stroke.unwrap() } else { egui::Stroke::NONE }));   
                self.add(Shape::line(points, arc.stroke));
            },

            PieMenuHighlightShape::ArcCircle => {                
                self.draw_arc(arc_values.unwrap());
                self.draw_circle(circle_values.unwrap());
            },

            PieMenuHighlightShape::ArcSliceCircle => {
                let arc = arc_values.unwrap();
                let slice = slice_values.unwrap();
                let points = self.arc_calculate_points(arc.center, arc.radius, arc.resolution, arc.angle_range);
                let mut slice_points = points.clone();
                slice_points.insert(0, arc.center);
                slice_points.push(arc.center);
                self.add(Shape::convex_polygon(slice_points, slice.fill_color, if slice.stroke.is_some() { slice.stroke.unwrap() } else { egui::Stroke::NONE }));   
                self.add(Shape::line(points, arc.stroke));
                self.draw_circle(circle_values.unwrap());
            },

            PieMenuHighlightShape::SliceCircle => {            
                let slice = slice_values.unwrap();
                assert!(slice.arc_values.is_some(), "ArcValues must be provided for slice highlight");
                let arc = slice.arc_values.unwrap();
                let mut points = self.arc_calculate_points(arc.center, arc.radius, arc.resolution, arc.angle_range);
                points.insert(0, arc.center);
                points.push(arc.center);
                self.add(Shape::convex_polygon(points, slice.fill_color, if slice.stroke.is_some() { slice.stroke.unwrap() } else { egui::Stroke::NONE }));            
                self.draw_circle(circle_values.unwrap());
                
            },
            PieMenuHighlightShape::None => {
                // Do nothing
            }
        }
    }
}


/* // Extension trait to add an arc drawing method to egui::Painter
pub trait ArcPainter {
    fn arc(&self, center: Pos2, radius: f32, angle_range: std::ops::Range<f32>, stroke: egui::Stroke);
}
impl ArcPainter for egui::Painter {
    fn arc(&self, 
        center: Pos2, 
        radius: f32,
        resolution: f32,
        width_angle_radians: f32,
        stroke: egui::Stroke) {
        
        let start_angle = width_angle_radians / -2;
        let arc_length = width_angle_radians * radius;
        let n_points = (arc_length / resolution).ceil() as usize;
        let step = width_angle_radians / n_points as f32;
        let points: Vec<Pos2> = (0..=n_points)
            .map(|i| {
                let angle = start_angle + i as f32 * step;
                center + egui::vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        self.add(Shape::line(points, stroke));
    }
} */