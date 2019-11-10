use std::cmp::{max, min};
use fanspeedcurve::FanspeedCurve;

const FLICKER_TEMP_MAX: i32 = 75;
const FLICKER_TEMP_MAX_REASON: &str = "\n        \
below the limit speed adjustments are not instant, so the temperature might temporarily rise higher";

pub struct FanFlickerRange {
    pub minimum_allowed: i32,
    pub fickering_starts: i32
}

pub struct FanFlickerFix {
    range: FanFlickerRange,
    previous_speed: i32,
}

impl FanFlickerRange {

    pub fn new(
        range: (u16, u16),
        curve: &FanspeedCurve,
        limits: &Option<(u16, u16)>,
    ) -> Result<FanFlickerRange, String> {

        let minimum_allowed = range.0;
        let fickering_starts = range.1;

        let errmsg = match ((minimum_allowed, fickering_starts), limits, curve.temp_x(fickering_starts)) {
            ((m, _), _, _) if m < 1 =>
                format!("fanflicker: `minimum` must be greater than zero"),
            ((m, s), _, _) if m >= s =>
                format!("fanflicker: `minimum` ({}) not less than `starts` ({})",
                        minimum_allowed, fickering_starts),
            (_, &Some((low, high)), _) if minimum_allowed < low || fickering_starts > high =>
                format!("fanflicker range [{}, {}] not within general fan limits [{}, {}]",
                        minimum_allowed, fickering_starts, low, high),
            (_, _, Some(speed)) if speed > FLICKER_TEMP_MAX =>
                format!("fanflicker: upper fanspeed limit of {} allows a \
                         temperature of {}°C which exceeds the safe limit of {}°C:{}",
                        fickering_starts, speed, FLICKER_TEMP_MAX, FLICKER_TEMP_MAX_REASON),
            (_, _, None) =>
                format!("fanflicker: upper fanspeed limit of {} is unreachable with the given points, \
                        so the safe temperature limit of {}°C can not be guaranteed:{}",
                        fickering_starts, FLICKER_TEMP_MAX, FLICKER_TEMP_MAX_REASON),
             _ => String::new(),
        };

        if errmsg.len() > 0 {
            return Err(errmsg);
        }

        info!("Trying to prevent fan flickering in range [{}, {}]", minimum_allowed, fickering_starts);

        Ok(FanFlickerRange {
            minimum_allowed: minimum_allowed as i32,
            fickering_starts: fickering_starts  as i32
        })
    }
}

impl FanFlickerFix {

    pub fn new(range: FanFlickerRange, previous_speed: i32) -> FanFlickerFix {
        debug!("FanFlickerFix: setting previous speed to {}%", previous_speed);
        FanFlickerFix { range, previous_speed }
    }

    pub fn minimum(&self) -> i32 {
        self.range.minimum_allowed
    }

    /// See if the `requested` new speed might trigger fan flicker. If so, modify it, taking
    /// the previously set speed into account.
    /// This will only have an effect when `requested` is at or below the `fickering_starts`
    /// value specified in `FanFlickerFix` or rpm is zero.
    #[deny(unreachable_patterns)]
    pub fn fix_speed(&mut self, current_rpm: i32, requested: i32) -> i32 {

        let fickering_starts = self.range.fickering_starts;
        let minimum_allowed = self.range.minimum_allowed;

        //  Currently flickering on and off.
        // TODO: history of previous attemps, maybe fickering_starts + 10 is required
        if current_rpm == 0 {
            self.previous_speed = fickering_starts;
            debug!("FanFlickerFix: flicking detected (RPM: 0), setting {}%",
                   self.previous_speed);
            return self.previous_speed;
        }


        let increment = 2;
        let decrement = 1;

        /// Position relative to the flicker range.
        #[derive(Debug)]
        enum Pos {
            Above(i32),
            InRange(i32),
            Below(i32), // i32 value only used for debug output.
        }

        /// Direction of speed change relative to previous state: increase or decrease.
        #[derive(Debug)]
        enum Dir {
            Inc,
            Dec, // also covers equality case
        }

        struct Delta {
            from: Pos,
            to: Pos,
            dir: Dir,
        }

        let mk_pos = |speed: i32| -> Pos {
            if speed < minimum_allowed {
                Pos::Below(speed)
            } else if speed > fickering_starts {
                Pos::Above(speed)
            } else {
                Pos::InRange(speed)
            }
        };

        let delta = Delta {
            from : mk_pos(self.previous_speed),
            to: mk_pos(requested),
            dir: if requested > self.previous_speed { Dir::Inc } else { Dir::Dec }
        };

        let new_speed = match delta {
            // Only watch over the flicker range.
            Delta { from: _, to: Pos::Above(new), dir: _ }
                => new,

            // Should never be in or come from Below (rpm == 0 caught above), but just in case...
            Delta { from: Pos::Below(_), to: _, dir: _ }
                => fickering_starts,

            // These from/to and direction combinations are impossible.
            Delta { from: Pos::Above(_), to: _, dir: Dir::Inc }
            | Delta { from: Pos::InRange(_), to: Pos::Below(_), dir: Dir::Inc }
                => unreachable!(),

            // Jumping down into the flicker range, possibly even below it.
            Delta { from: Pos::Above(_), to: Pos::InRange(_), dir: Dir::Dec }
            | Delta { from: Pos::Above(_), to: Pos::Below(_), dir: Dir::Dec }
                => fickering_starts,

            // Speed increases also cause this problem, so slow the speedup,
            // sometimes leads to flickering nonetheless, not reliable.
            // When slowing the increase the here the temperature might reach FLICKER_TEMP_MAX.
            Delta { from: Pos::InRange(prev), to: Pos::InRange(new), dir: Dir::Inc }
                => min(prev + increment, new),

            // Lowering the speed and between minimum_allowed and fickering_starts, step one down.
            Delta { from: Pos::InRange(prev), to: Pos::InRange(new), dir: Dir::Dec }
                => max(prev - decrement, new),

            // Lowering below the range requested.
            Delta { from: Pos::InRange(prev), to: Pos::Below(_), dir: Dir::Dec }
                => max(prev - decrement, minimum_allowed),
        };

        debug!("FanFlickerFix [{}, {}]: requested change from {:?} to {:?} ({}), {} {}%",
               minimum_allowed,
               fickering_starts,
               delta.from,
               delta.to,
               if let Dir::Inc = delta.dir { "increase" } else { "decrease" },
               if new_speed == self.previous_speed { "staying at" } else { "setting" },
               new_speed);

        self.previous_speed = new_speed;

        new_speed
    }
}
