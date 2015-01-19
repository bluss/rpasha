use std::num::Float;

/// Smooth out the vision circle by fuzzing the radius a bit
const RADIUS_FUDGE: f32 = 0.33;

/// Squares marked not visible act as obstacles.
const NONVIS_OCCLUDE: bool = true;

/// You may choose to have include or exclude the end points here,
/// `<` is more permissive than `<=`.
fn angle_contained_in(angle: f32, start: f32, end: f32) -> bool
{
    start < angle && angle < end
}

/// Decide visibility of the square based on center, near, and far corner visibility
fn visible_when(center: bool, near: bool, far: bool) -> bool
{
    center && (near || far)
}

/// Restrictive Precise Angle Shadowcasting.
///
/// A port of https://github.com/MoyTW/roguebasin_rpas to Rust.
///
/// **RPAShadowcasting\<F\>** is an iterator that iterates a circular
/// region. It always uses coordinates centered on **(0, 0)** and yields
/// tuples **(i32, i32, bool)** representing the offset relative to the origin,
/// and a boolean for the square's visibility.
///
/// All squares inside the radius are yielded. The origin is always yielded first
/// with visibility **true**.
pub struct RPAShadowcasting<F> {
    obstruct: F,
    obstructions: Vec<(f32, f32)>,
    /// identify x-y quadrant and top/bottom half of quadrant.
    /// cycle (1, 1) -> (1, -1) -> (-1, -1) -> (-1, 1)
    octant: (i32, i32, bool),
    /// size of the circle
    radius: i32,
    /// radial coordinate
    r: i32,
    /// transversal coordinate
    x: i32,
}

impl<F> RPAShadowcasting<F> where
    F: FnMut(i32, i32) -> bool,
{
    /// Create a new **RPAShadowcasting**.
    ///
    /// The function **obstruct(x, y)** should return **true**
    /// if the relative coordinates **x, y** are obstructing vision.
    pub fn new(radius: i32, obstruct: F) -> Self {
        RPAShadowcasting {
            obstruct: obstruct,
            obstructions: Vec::new(),
            octant: (1, 1, true),
            radius: radius,
            r: 0,
            x: 0,
        }
    }

    fn next_octant(&mut self)
    {
        let (ref mut x, ref mut y, ref mut vert) = self.octant;
        *vert = !*vert;
        if !*vert {
            return;
        }
        if *x > 0 && *y > 0 {
            *y = -1;
        } else if *x > 0 && *y < 0 {
            *x = -1;
        } else if *x < 0 && *y < 0 {
            *y = 1;
        } else {
            *x = 1;
        }
    }
}

impl<F> Iterator for RPAShadowcasting<F> where
    F: FnMut(i32, i32) -> bool,
{
    /// The iterator element represents **(x, y, visible)** in coordinates
    /// relative to the center.
    type Item = (i32, i32, bool);

    /// Algorithm in very brief summary:
    ///
    /// Treat each octant wedge completely separately.
    /// List occlusions as (start, end) angle for the current octant wedge.
    ///
    /// Visit squares by radial coordinate r in 1...radius and transversal
    /// coordinate x in 1...r.
    ///
    /// ```ignore
    ///  |  .    
    ///  |  12
    ///  v  345     1,2,.. is visit order
    ///  r  6789
    ///     -->x
    /// ```
    ///
    /// Compute angles to the square's near, center
    /// and far edge and compare with all previous recorded occlusions.
    fn next(&mut self) -> Option<(i32, i32, bool)>
    {
        if self.r == 0 {
            self.r += 1;
            return Some((0, 0, true));
        }
        if self.x > self.r {
            self.x = 0;
            self.r += 1;
        }

        // Skip to next octant when we reach the radial limit.
        if self.r > self.radius {
            self.next_octant();
            self.x = 0;
            self.r = 1;
            self.obstructions.clear();
            if self.octant == (1,1,true) {
                // back at the original octant and done.
                return None;
            } else {
                return self.next();
            }
        }

        let (qx, qy, vert) = self.octant;

        let (a, b) = if vert {
            (self.x * qx, self.r * qy)
        } else {
            (self.r * qx, self.x * qy)
        };

        if Float::hypot(a as f32, b as f32) >= RADIUS_FUDGE + self.radius as f32 {
            self.x += 1;
            return self.next();
        }

        let angle_alloc = 1. / ((self.r + 1) as f32);
        let near_angle = (self.x as f32) * angle_alloc;
        let center_angle = near_angle + 0.5 * angle_alloc;
        let far_angle = near_angle + angle_alloc;

        // check visibility vs prev obstructions.
        let mut visible = true;
        let mut near_vis = true;
        let mut center_vis = true;
        let mut far_vis = true;
        for &(near_obs, far_obs) in self.obstructions.iter() {
            near_vis = near_vis && !angle_contained_in(near_angle, near_obs, far_obs);
            center_vis = center_vis && !angle_contained_in(center_angle, near_obs, far_obs);
            far_vis = far_vis && !angle_contained_in(far_angle, near_obs, far_obs);

            visible = visible_when(center_vis, near_vis, far_vis);
            if !visible {
                break;
            }
        }
        if (NONVIS_OCCLUDE && !visible) || (self.obstruct)(a, b) {
            self.obstructions.push((near_angle, far_angle));
        }
        self.x += 1;
        Some((a, b, visible))
    }
}
