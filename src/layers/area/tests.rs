use crate::projection::{GeoPos, MapProjection};
use egui::{pos2, vec2, Color32, Pos2, Rect, Stroke};

use super::layer::AreaLayer;
use super::types::{Area, AreaMode, AreaShape, DraggedObject};

// Helper for creating a dummy projection for tests
fn dummy_projection() -> MapProjection {
    MapProjection::new(
        10,                // zoom
        (0.0, 0.0).into(), // center
        Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1000.0)),
    )
}

#[test]
fn area_layer_new() {
    let layer = AreaLayer::default();
    assert_eq!(layer.mode, AreaMode::Disabled);
    assert!(layer.areas.is_empty());
    assert_eq!(layer.node_radius, 5.0);
}

#[test]
fn area_layer_add_area() {
    let mut layer = AreaLayer::default();
    assert_eq!(layer.areas.len(), 0);

    layer.add_area(Area {
        shape: AreaShape::Polygon(vec![
            (0.0, 0.0).into(),
            (1.0, 0.0).into(),
            (0.0, 1.0).into(),
        ]),
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    });

    assert_eq!(layer.areas.len(), 1);
}

#[test]
fn circle_get_points_with_fixed_number() {
    let projection = dummy_projection();
    let area = Area {
        shape: AreaShape::Circle {
            center: (0.0, 0.0).into(),
            radius: 1000.0,
            points: Some(16),
        },
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    };

    let points = area.get_points(&projection);
    assert_eq!(points.len(), 16);
}

#[test]
fn find_object_at_empty() {
    let layer = AreaLayer::default();
    let projection = dummy_projection();
    let position = pos2(100.0, 100.0);

    assert!(layer.find_object_at(position, &projection, None).is_none());
}

#[test]
fn find_object_at_polygon_node() {
    let projection = dummy_projection();
    let mut layer = AreaLayer::default();
    let geo_pos = projection.unproject(pos2(100.0, 100.0));

    layer.add_area(Area {
        shape: AreaShape::Polygon(vec![geo_pos]),
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    });

    // Position is exactly on the node
    let found = layer.find_object_at(pos2(100.0, 100.0), &projection, None);
    assert!(matches!(
        found,
        Some(DraggedObject::PolygonNode {
            area_index: 0,
            node_index: 0
        })
    ));

    // Position is slightly off but within tolerance
    let found_nearby = layer.find_object_at(pos2(101.0, 101.0), &projection, None);
    assert!(matches!(
        found_nearby,
        Some(DraggedObject::PolygonNode {
            area_index: 0,
            node_index: 0
        })
    ));

    // Position is too far
    let not_found = layer.find_object_at(pos2(200.0, 200.0), &projection, None);
    assert!(not_found.is_none());
}

#[test]
fn area_layer_serde() {
    let mut layer = AreaLayer::default();
    layer.add_area(Area {
        shape: AreaShape::Polygon(vec![(0.0, 0.0).into()]),
        stroke: Stroke::new(1.0, Color32::RED),
        fill: Color32::BLUE,
        fill_type: Default::default(),
    });

    let json = serde_json::to_string(&layer).unwrap();
    let deserialized: AreaLayer = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.areas.len(), 1);
    assert_eq!(deserialized.mode, AreaMode::Disabled); // Restored to default
}

#[test]
fn test_can_triangulate_valid() {
    let projection = dummy_projection();
    let area = Area {
        shape: AreaShape::Polygon(vec![
            (0.0, 0.0).into(),
            (10.0, 0.0).into(),
            (0.0, 10.0).into(),
        ]),
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    };

    assert!(area.can_triangulate(&projection));
}

#[test]
fn test_can_triangulate_insufficient_points() {
    let projection = dummy_projection();
    let area = Area {
        shape: AreaShape::Polygon(vec![(0.0, 0.0).into(), (10.0, 0.0).into()]),
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    };

    // Should return true as we don't consider < 3 points as a triangulation failure
    // (it simply doesn't draw anything)
    assert!(area.can_triangulate(&projection));
}

#[cfg(feature = "geojson")]
mod geojson_tests {
    use super::*;

    #[test]
    fn area_layer_geojson_polygon() {
        let mut layer = AreaLayer::default();
        layer.add_area(Area {
            shape: AreaShape::Polygon(vec![
                (10.0, 20.0).into(),
                (30.0, 40.0).into(),
                (50.0, 60.0).into(),
            ]),
            stroke: Stroke::new(2.0, Color32::from_rgb(0, 0, 255)),
            fill: Color32::from_rgba_unmultiplied(255, 0, 0, 128),
            fill_type: Default::default(),
        });

        let geojson_str = layer.to_geojson_str().unwrap();

        let mut new_layer = AreaLayer::default();
        new_layer.from_geojson_str(&geojson_str).unwrap();

        assert_eq!(new_layer.areas.len(), 1);
        assert_eq!(layer.areas[0], new_layer.areas[0]);
    }

    #[test]
    fn area_layer_geojson_circle() {
        let mut layer = AreaLayer::default();
        layer.add_area(Area {
            shape: AreaShape::Circle {
                center: (10.0, 20.0).into(),
                radius: 1000.0,
                points: Some(32),
            },
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        });

        let geojson_str = layer.to_geojson_str().unwrap();
        let mut new_layer = AreaLayer::default();
        new_layer.from_geojson_str(&geojson_str).unwrap();

        assert_eq!(new_layer.areas.len(), 1);
        assert_eq!(layer.areas[0].shape, new_layer.areas[0].shape);
    }

    #[test]
    fn area_layer_geojson_ellipse() {
        let mut layer = AreaLayer::default();
        layer.add_area(Area {
            shape: AreaShape::Ellipse {
                center: (10.0, 20.0).into(),
                radius_major: 2000.0,
                radius_minor: 1000.0,
                rotation: 0.78,
                points: Some(32),
            },
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        });

        let geojson_str = layer.to_geojson_str().unwrap();
        let mut new_layer = AreaLayer::default();
        new_layer.from_geojson_str(&geojson_str).unwrap();

        assert_eq!(new_layer.areas.len(), 1);
        assert_eq!(layer.areas[0].shape, new_layer.areas[0].shape);
    }
}

#[test]
fn ellipse_get_points_with_fixed_number() {
    let projection = dummy_projection();
    let area = Area {
        shape: AreaShape::Ellipse {
            center: (0.0, 0.0).into(),
            radius_major: 2000.0,
            radius_minor: 1000.0,
            rotation: 0.5,
            points: Some(24),
        },
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    };

    let points = area.get_points(&projection);
    assert_eq!(points.len(), 24);
}

#[test]
fn ellipse_containment() {
    let projection = dummy_projection();
    let area = Area {
        shape: AreaShape::Ellipse {
            center: (0.0, 0.0).into(),
            radius_major: 2000.0,
            radius_minor: 1000.0,
            rotation: 0.0,
            points: None,
        },
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    };

    // Center is inside
    assert!(area.contains(projection.project((0.0, 0.0).into()), &projection));

    // Slightly to the East (lon changes) should be inside
    let point_inside = GeoPos {
        lon: 0.005,
        lat: 0.0,
    };
    assert!(area.contains(projection.project(point_inside), &projection));

    // Very far to the East (lon changes) should be outside
    let point_outside = GeoPos { lon: 0.5, lat: 0.0 };
    assert!(!area.contains(projection.project(point_outside), &projection));
}

#[test]
fn find_node_at_on_segment() {
    let projection = dummy_projection();
    let mut layer = AreaLayer::default();

    let p1 = projection.unproject(pos2(100.0, 100.0));
    let p2 = projection.unproject(pos2(200.0, 100.0));

    layer.add_area(Area {
        shape: AreaShape::Polygon(vec![p1, p2, projection.unproject(pos2(150.0, 200.0))]), // Triangle
        stroke: Default::default(),
        fill: Default::default(),
        fill_type: Default::default(),
    });

    // Click exactly between p1 and p2
    let click_pos = pos2(150.0, 100.0);

    // Should NOT find a node
    assert!(layer.find_node_at(click_pos, &projection, None).is_none());

    // Should find the segment
    let segment = layer.find_line_segment_at(click_pos, &projection, None);
    assert!(segment.is_some());
    assert_eq!(segment.unwrap().0, 0); // area_index
    assert_eq!(segment.unwrap().1, 0);
}
