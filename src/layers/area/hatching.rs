use egui::Pos2;

/// Clips a line segment to a rectangle using the Liang-Barsky algorithm.
pub(crate) fn clip_segment_to_rect(p1: Pos2, p2: Pos2, rect: egui::Rect) -> Option<(Pos2, Pos2)> {
    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;

    let checks = [
        (-dx, p1.x - rect.min.x), // Left
        (dx, rect.max.x - p1.x),  // Right
        (-dy, p1.y - rect.min.y), // Top
        (dy, rect.max.y - p1.y),  // Bottom
    ];

    for &(p, q) in &checks {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }

    if t0 <= t1 {
        Some((
            Pos2::new(p1.x + t0 * dx, p1.y + t0 * dy),
            Pos2::new(p1.x + t1 * dx, p1.y + t1 * dy),
        ))
    } else {
        None
    }
}

/// Generates diagonal hatching line segments clipped to the given polygon and optionally a viewport.
///
/// `screen_points` are the polygon vertices in screen space (must be >= 3 points).
/// `spacing` is the distance in pixels between parallel hatching lines.
/// `angle` is the angle of the hatching lines in radians (0 = horizontal, PI/4 = 45° diagonal).
/// `viewport` is an optional bounding rectangle; if provided, hatching is only generated and clipped
/// within this viewport.
///
/// Returns a list of line segments `(start, end)` that lie inside the polygon and viewport.
pub(crate) fn generate_hatching_lines(
    screen_points: &[Pos2],
    spacing: f32,
    angle: f32,
    viewport: Option<egui::Rect>,
) -> Vec<(Pos2, Pos2)> {
    if screen_points.len() < 3 || spacing <= 0.0 {
        return Vec::new();
    }

    // Direction along the hatching lines and perpendicular to them.
    let dir = egui::vec2(angle.cos(), angle.sin());
    let perp = egui::vec2(-angle.sin(), angle.cos());

    // Project all polygon points onto the perpendicular axis to find the sweep range.
    let mut min_perp = f32::MAX;
    let mut max_perp = f32::MIN;
    for p in screen_points {
        let d = p.to_vec2().dot(perp);
        min_perp = min_perp.min(d);
        max_perp = max_perp.max(d);
    }

    // Project viewport corners onto the perpendicular axis to limit the sweep range.
    if let Some(rect) = viewport {
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
        ];
        let mut min_view_perp = f32::MAX;
        let mut max_view_perp = f32::MIN;
        for c in corners {
            let d = c.to_vec2().dot(perp);
            min_view_perp = min_view_perp.min(d);
            max_view_perp = max_view_perp.max(d);
        }
        min_perp = min_perp.max(min_view_perp);
        max_perp = max_perp.min(max_view_perp);
    }

    let n = screen_points.len();
    let mut segments = Vec::new();

    // Sweep parallel lines across the polygon.
    let mut offset = min_perp + spacing;
    while offset < max_perp {
        // A point on the current sweep line: origin + offset along the perpendicular.
        let line_origin = Pos2::ZERO + perp * offset;

        // Find intersections of this sweep line with every polygon edge.
        let mut t_values: Vec<f32> = Vec::new();
        for i in 0..n {
            let a = screen_points[i];
            let b = screen_points[(i + 1) % n];
            let edge = b - a;

            // Solve: a + t_edge * edge = line_origin + t_line * dir
            // Cross product form: (a - line_origin) × dir = t_edge * (edge × dir)
            let denom = edge.x * dir.y - edge.y * dir.x;
            if denom.abs() < 1e-9 {
                continue; // Edge is parallel to the hatching line.
            }

            let diff = a - line_origin;
            let t_edge = -(diff.x * dir.y - diff.y * dir.x) / denom;

            if (0.0..=1.0).contains(&t_edge) {
                // Compute t_line: the parameter along the sweep line direction.
                let t_line = if dir.x.abs() > dir.y.abs() {
                    (a.x - line_origin.x + t_edge * edge.x) / dir.x
                } else {
                    (a.y - line_origin.y + t_edge * edge.y) / dir.y
                };
                t_values.push(t_line);
            }
        }

        // Sort intersections along the sweep line.
        t_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Pair up intersections (even-odd rule) to get interior segments.
        for pair in t_values.chunks_exact(2) {
            let p1 = line_origin + dir * pair[0];
            let p2 = line_origin + dir * pair[1];
            if let Some(rect) = viewport {
                if let Some((clipped_p1, clipped_p2)) = clip_segment_to_rect(p1, p2, rect) {
                    segments.push((clipped_p1, clipped_p2));
                }
            } else {
                segments.push((p1, p2));
            }
        }

        offset += spacing;
    }

    segments
}
