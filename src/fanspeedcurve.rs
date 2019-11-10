#[derive(Debug, PartialEq)]
pub struct FanspeedCurve(Vec<(u16, u16)>);

const EPTS: &'static str = "not enough data points";
const EMONO: &'static str = "not monotonically increasing";


//         ^
//         |                             (3) +--------------->
//      f  |                                /
//      a  |                               /
//      n  |                           _.-+
//      s  |                     (2) +`
//      p  |
//      e  |
//      e  |                  +------o
//      d  |                 /
//         |                /
//      %  |               /
//         |          (1) +
//         |
//         +--------------------------------------------------------->
//
//            temperature (°C)
//
// Temperature (x) to fanspeed (y) curve. `speed_y` will return `None` below
// the minimum (1). Discontinuities are possible (2), then the larger value is used.
// For values larger than the maximum (3), the maximum is returned.
// If quering the corresponding temperature for a given speed (`temp_x`), `None` is
// returned both below and above the minimum or maximum fanspeed.

impl FanspeedCurve {

    pub fn new(points: Vec<(u16, u16)>) -> Result<FanspeedCurve, &'static str> {
        if points.len() <= 1 {
            Err(EPTS)
        } else if !points.windows(2).all(|pair| pair[0].0 <= pair[1].0 && pair[0].1 <= pair[1].1) {
            Err(EMONO)
        } else {
            Ok(FanspeedCurve(remove_redundant_points(points)))
        }
    }

     pub fn minspeed(&self) -> i32 {
        self.0.first().unwrap().1 as i32
    }

    pub fn speed_y(&self, temp_x: u16) -> Option<i32> {

        let last = self.0.last().unwrap();
        // `>=` to prevent dx = 0 and division by zero if p0/p1 have equal x values
        if temp_x >= last.0 {
            debug!("Temperature outside curve; setting to max");
            return Some(last.1 as i32)
        }

        if temp_x < self.0.first().unwrap().0 {
            return None
        }

        // `rev()` so dx is always > 0, i.e. the slope of a purely vertical
        // point pair is never calculated because the endpoint of the previous one
        // matched already or was handled above if this is the last pair.
        for i in self.0.windows(2).rev() {
            let (p0, p1) = (i[0], i[1]);

            if temp_x >= p0.0 && temp_x <= p1.0 {
                let dx = p1.0 - p0.0;
                let dy = p1.1 - p0.1;

                if dx == 0 {
                   unreachable!();
                }

                let slope = (dy as f32) / (dx as f32);

                let speed_y = (p0.1 as f32) + (((temp_x - p0.0) as f32) * slope);

                return Some(speed_y as i32)
            }
        }

        // <min and >max were previously handled
        unreachable!()
    }

    pub fn temp_x(&self, speed_y: u16) -> Option<i32> {

        let last = self.0.last().unwrap();
        // to prevent dy = 0 and division by zero if p0/p1 have equal y values
        if speed_y == last.1 {
            return Some(last.0 as i32)
        }

        for i in self.0.windows(2).rev() { // `rev()`, see above
            let (p0, p1) = (i[0], i[1]);

            if speed_y >= p0.1 && speed_y <= p1.1 {
                let dx = p1.0 - p0.0;
                let dy = p1.1 - p0.1;

                if dy == 0 {
                   unreachable!();
                }

                let slope = (dx as f32) / (dy as f32);

                let temp_x = (p0.0 as f32) + (((speed_y - p0.1) as f32) * slope);

                return Some(temp_x as i32)
            }
        }

        None
    }
}

fn remove_redundant_points(points: Vec<(u16, u16)>) -> Vec<(u16, u16)> {

    let three_identical_x_or_y_coords = |x3: &[(usize, &(u16, u16))]| -> bool {
        ((x3[0].1).0 == (x3[1].1).0 && (x3[0].1).0 == (x3[2].1).0)
        || ((x3[0].1).1 == (x3[1].1).1 && (x3[0].1).1 == (x3[2].1).1)
    };

    let remove_indices = points
        .iter()
        .enumerate()
        .collect::<Vec<_>>()
        .windows(3)
        .filter_map(|x| {
            if three_identical_x_or_y_coords(x) {
                let index_of_middle_point = x[1].0;
                Some(index_of_middle_point)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    points
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if remove_indices.iter().find(|x| **x == index).is_some() {
                None
            } else {
                Some(*value)
            }
        })
        .collect::<Vec<_>>()
}

#[test]
fn test_remove_redundant_points() {
    let p = vec![(1, 1),
                 (2, 2), (2, 5), (2, 5),
                 (3, 8), (3, 9), (3, 10),
                 (4, 10),
                 (5, 11), (6, 11), (7, 11),
                 (8, 12)];

    let q = remove_redundant_points(p);

    assert_eq!(q, vec![(1, 1), (2, 2), (2, 5), (3, 8), (3, 10),
                       (4, 10), (5, 11), (7, 11), (8, 12)]);

    assert!(FanspeedCurve::new(q).is_ok());
}

#[test]
fn test_empty() {
    assert_eq!(FanspeedCurve::new(vec![]).err(), Some(EPTS));
}

#[test]
fn test_dot_only() {
    assert_eq!(FanspeedCurve::new(vec![(4, 6),]).err(), Some(EPTS));
}

#[test]
fn test_decreasing() {
    let down = FanspeedCurve::new(vec![(0, 10), (2, 5), (3, 1)]);

    assert_eq!(down.err(), Some(EMONO));
}

#[test]
fn test_non_monotonic() {
    let up_down = FanspeedCurve::new(vec![(0, 0), (50, 20), (100, 10)]);

    assert_eq!(up_down.err(), Some(EMONO));
}

#[test]
fn test_single_slope() {
    let single_slope = FanspeedCurve::new(vec![(5, 0), (105, 20),]).unwrap();

    assert_eq!(single_slope.speed_y(0), None);
    assert_eq!(single_slope.speed_y(3), None);
    assert_eq!(single_slope.speed_y(5 + 0), Some(0));
    assert_eq!(single_slope.speed_y(5 + 25), Some(5));
    assert_eq!(single_slope.speed_y(5 + 50), Some(10));
    assert_eq!(single_slope.speed_y(5 + 75), Some(15));
    assert_eq!(single_slope.speed_y(5 + 100), Some(20));
    assert_eq!(single_slope.speed_y(5 + 101), Some(20));
    assert_eq!(single_slope.speed_y(10101), Some(20));

    assert_eq!(single_slope.temp_x(0), Some(5 + 0));
    assert_eq!(single_slope.temp_x(5), Some(5 + 25));
    assert_eq!(single_slope.temp_x(10), Some(5 + 50));
    assert_eq!(single_slope.temp_x(15), Some(5 + 75));
    assert_eq!(single_slope.temp_x(20), Some(5 + 100));
    assert_eq!(single_slope.temp_x(21), None);

    assert_eq!(single_slope.minspeed(), 0);
}

#[test]
fn test_multiple_values() {
    let multiple = FanspeedCurve::new(vec![(0, 1), (5, 10), (10, 60)]).unwrap();

    assert_eq!(multiple.speed_y(0), Some(1));
    assert_eq!(multiple.speed_y(5), Some(10));
    assert_eq!(multiple.speed_y(10), Some(60));
    assert_eq!(multiple.speed_y(11), Some(60));
    assert_eq!(multiple.speed_y(101), Some(60));

    assert_eq!(multiple.temp_x(0), None);
    assert_eq!(multiple.temp_x(1), Some(0));
    assert_eq!(multiple.temp_x(10), Some(5));
    assert_eq!(multiple.temp_x(20), Some(6));
    assert_eq!(multiple.temp_x(50), Some(9));
    assert_eq!(multiple.temp_x(60), Some(10));
    assert_eq!(multiple.temp_x(61), None);

    assert_eq!(multiple.minspeed(), 1);
}

#[test]
fn test_horizontal() {
    let horizon = FanspeedCurve::new(vec![(20, 35), (22, 35), (25, 35), (60, 35)]);

    assert!(horizon.is_ok());
    let horizon = horizon.unwrap();

    assert_eq!(horizon.speed_y(19), None);
    assert_eq!(horizon.speed_y(20), Some(35));
    assert_eq!(horizon.speed_y(21), Some(35));
    assert_eq!(horizon.speed_y(59), Some(35));
    assert_eq!(horizon.speed_y(60), Some(35));
    assert_eq!(horizon.speed_y(61), Some(35));

    assert_eq!(horizon.temp_x(34), None);
    assert_eq!(horizon.temp_x(35), Some(60));
    assert_eq!(horizon.temp_x(36), None);
}

#[test]
fn test_vertical() {
    let vertical = FanspeedCurve::new(vec![(20, 5), (20, 10), (20, 50), (20, 55)]);

    assert!(vertical.is_ok());
    let vertical = vertical.unwrap();

    assert_eq!(vertical.speed_y(19), None);
    assert_eq!(vertical.speed_y(20), Some(55));
    assert_eq!(vertical.speed_y(21), Some(55));

     assert_eq!(vertical.temp_x(4), None);
     assert_eq!(vertical.temp_x(5), Some(20));
     assert_eq!(vertical.temp_x(6), Some(20));
     assert_eq!(vertical.temp_x(54), Some(20));
     assert_eq!(vertical.temp_x(55), Some(20));
     assert_eq!(vertical.temp_x(56), None);
}


#[test]
fn test_plateau_values() {
    let plateau = FanspeedCurve::new(vec![(0, 0), (10, 50), (20, 50), (30, 100)]);

    assert!(plateau.is_ok());
    let plateau = plateau.unwrap();

    assert_eq!(plateau.speed_y(10), Some(50));
    assert_eq!(plateau.speed_y(15), Some(50));
    assert_eq!(plateau.speed_y(20), Some(50));

    assert_eq!(plateau.temp_x(50), Some(20));
}

#[test]
fn test_cliff_values() {
    let cliff = FanspeedCurve::new(vec![(5, 5), (10, 20),  (10, 40), (10, 50), (30, 90)]);

    assert!(cliff.is_ok());
    let cliff = cliff.unwrap();

    assert_eq!(cliff.speed_y(4), None);
    assert_eq!(cliff.speed_y(10), Some(50));
    assert_eq!(cliff.speed_y(30), Some(90));
    assert_eq!(cliff.speed_y(31), Some(90));

    assert_eq!(cliff.temp_x(20), Some(10));
    assert_eq!(cliff.temp_x(43), Some(10));
    assert_eq!(cliff.temp_x(50), Some(10));
    assert_eq!(cliff.temp_x(90), Some(30));
}

#[test]
fn test_stairs() {
    let stairs = FanspeedCurve::new(
        vec![(10, 1), (10, 5), (10, 10), (20, 10), (20, 20), (30, 20), (30, 30), (30, 40)]);

    assert!(stairs.is_ok());
    let stairs = stairs.unwrap();

    assert_eq!(stairs.speed_y(30), Some(40));

    assert_eq!(stairs.speed_y(9), None);
    assert_eq!(stairs.speed_y(10), Some(10));
    assert_eq!(stairs.speed_y(11), Some(10));
    assert_eq!(stairs.speed_y(19), Some(10));
    assert_eq!(stairs.speed_y(20), Some(20));
    assert_eq!(stairs.speed_y(20), Some(20));
    assert_eq!(stairs.speed_y(21), Some(20));
    assert_eq!(stairs.speed_y(29), Some(20));
    assert_eq!(stairs.speed_y(31), Some(40));
    assert_eq!(stairs.speed_y(60), Some(40));
}

