//! The forward pass: pulses travel the connections one after the other and each layer lights
//! up as they arrive (`ManimML`'s `make_forward_pass_animation`).

/// A forward pass from `start` over `duration` seconds. Each connection gets an equal part of
/// the time; the pulses reach the last layer at `start + duration`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForwardPass {
    pub start: f32,
    pub duration: f32,
}

impl Default for ForwardPass {
    fn default() -> Self {
        Self {
            start: 0.5,
            duration: 3.0,
        }
    }
}

impl ForwardPass {
    /// How far the pulses are at `t`, in connections: 0 at the first layer, `gaps` (the
    /// number of connections) at the last.
    pub fn front(&self, t: f32, gaps: usize) -> f32 {
        if gaps == 0 || self.duration <= 0.0 {
            return if t >= self.start { gaps as f32 } else { 0.0 };
        }
        ((t - self.start) / self.duration * gaps as f32).clamp(0.0, gaps as f32)
    }

    /// When the pulses reach layer `layer` of a network with `gaps` connections.
    pub fn arrival(&self, layer: usize, gaps: usize) -> f32 {
        if gaps == 0 {
            return self.start;
        }
        self.start + self.duration * layer as f32 / gaps as f32
    }

    /// How brightly layer `layer` is lit at `t`, from 0 to 1: it brightens just before the
    /// pulses arrive and fades over the time of one connection after.
    pub fn glow(&self, t: f32, layer: usize, gaps: usize) -> f32 {
        let slot = if gaps == 0 {
            self.duration.max(0.3)
        } else {
            (self.duration / gaps as f32).max(1e-3)
        };
        let arrival = self.arrival(layer, gaps);
        if t < arrival {
            1.0 - ((arrival - t) / (0.15 * slot)).clamp(0.0, 1.0)
        } else {
            1.0 - ((t - arrival) / slot).clamp(0.0, 1.0)
        }
    }

    /// Where the pulses on connection `connection` are, from 0 (leaving) to 1 (arrived), or
    /// `None` while there are none on it.
    pub fn pulse(&self, t: f32, connection: usize, gaps: usize) -> Option<f32> {
        let along = self.front(t, gaps) - connection as f32;
        (along > 0.0 && along < 1.0).then_some(along)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pass_reaches_the_last_layer_at_its_end() {
        let pass = ForwardPass {
            start: 1.0,
            duration: 3.0,
        };
        assert_eq!(pass.front(0.5, 3), 0.0);
        assert_eq!(pass.front(2.0, 3), 1.0);
        assert_eq!(pass.front(4.0, 3), 3.0);
        assert_eq!(pass.front(9.0, 3), 3.0);
        assert_eq!(pass.arrival(3, 3), 4.0);
        assert_eq!(pass.glow(4.0, 3, 3), 1.0);
        assert_eq!(pass.glow(0.0, 3, 3), 0.0);
        // the first layer has faded by the time the pulses reach the second
        assert_eq!(pass.glow(2.0, 0, 3), 0.0);
    }

    #[test]
    fn one_connection_carries_pulses_at_a_time() {
        let pass = ForwardPass {
            start: 0.0,
            duration: 2.0,
        };
        assert_eq!(pass.pulse(0.5, 0, 2), Some(0.5));
        assert_eq!(pass.pulse(0.5, 1, 2), None);
        assert_eq!(pass.pulse(1.5, 1, 2), Some(0.5));
        assert_eq!(pass.pulse(2.5, 1, 2), None);
    }
}
