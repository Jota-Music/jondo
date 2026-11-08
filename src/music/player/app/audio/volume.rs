/// The slider is a 0..1 setting, but the low end needs a real quiet floor: a
/// linear bar still leaves the minimum too loud to use as an off switch. From
/// `QUIET_START` up it stays linear, so lowering is responsive and needs no
/// precision; below it a quadratic taper reaches true silence.
const QUIET_START: f32 = 0.3;
const QUIET_LEVEL: f32 = 0.2;

pub(super) fn gain(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value >= QUIET_START {
        QUIET_LEVEL + (value - QUIET_START) / (1.0 - QUIET_START) * (1.0 - QUIET_LEVEL)
    } else {
        let t = value / QUIET_START;
        t * t * QUIET_LEVEL
    }
}

#[cfg(test)]
#[path = "volume.test.rs"]
mod tests;
