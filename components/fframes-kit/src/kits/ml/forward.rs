//! The forward pass: pulses travel the connections one after the other and each layer lights
//! up as they arrive (`ManimML`'s `make_forward_pass_animation`), and dropout.

use crate::motion::Rng;

use super::layers::Layer;

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

/// Dropout (`ManimML`'s `make_neural_network_dropout_animation`): from `start` for `duration`
/// seconds a share `rate` (0 to 1) of the neurons of every hidden feed-forward layer is
/// crossed out and their edges fade. The neurons are picked once, the same for the same
/// `seed`; every layer keeps at least one, and the input and output layers keep all of theirs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dropout {
    pub rate: f32,
    pub seed: u64,
    pub start: f32,
    pub duration: f32,
}

impl Default for Dropout {
    fn default() -> Self {
        Self {
            rate: 0.3,
            seed: 1,
            start: 0.5,
            duration: 3.0,
        }
    }
}

/// How long the dropped neurons take to fade out and back, in seconds.
const DROPOUT_FADE: f32 = 0.2;

impl Dropout {
    /// How far the dropped neurons are gone at `t`, from 0 to 1.
    pub fn amount(&self, t: f32) -> f32 {
        let fade = DROPOUT_FADE.min(self.duration / 2.0).max(1e-3);
        let end = self.start + self.duration;
        ((t - self.start) / fade)
            .min((end - t) / fade)
            .clamp(0.0, 1.0)
    }

    /// Which neurons of each layer drop out: one list per layer, empty for the first and the
    /// last layer and for layers that are not feed-forward.
    pub fn mask(&self, layers: &[&Layer<'_>]) -> Vec<Vec<bool>> {
        let mut rng = Rng::new(self.seed);
        let last = layers.len().saturating_sub(1);
        layers
            .iter()
            .enumerate()
            .map(|(i, layer)| match layer {
                Layer::FeedForward { nodes, .. } if i > 0 && i < last => {
                    let mut dropped: Vec<bool> = (0..*nodes)
                        .map(|_| rng.unit() <= f64::from(self.rate))
                        .collect();
                    if dropped.iter().all(|&d| d) {
                        // keep the first
                        if let Some(first) = dropped.first_mut() {
                            *first = false;
                        }
                    }
                    dropped
                }
                _ => Vec::new(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropout_is_the_same_for_a_seed_and_spares_the_ends() {
        let layers = [
            Layer::feed_forward(4),
            Layer::feed_forward(8),
            Layer::MaxPool2D { kernel: 2 },
            Layer::feed_forward(8),
            Layer::feed_forward(3),
        ];
        let refs: Vec<&Layer<'_>> = layers.iter().collect();
        let dropout = Dropout {
            rate: 0.5,
            seed: 11,
            ..Default::default()
        };
        let mask = dropout.mask(&refs);
        assert_eq!(mask, dropout.mask(&refs));
        let other = Dropout {
            seed: 12,
            ..dropout
        }
        .mask(&refs);
        assert_ne!(mask, other);
        assert_eq!(mask[0], Vec::<bool>::new());
        assert_eq!(mask[2], Vec::<bool>::new());
        assert_eq!(mask[4], Vec::<bool>::new());
        assert_eq!((mask[1].len(), mask[3].len()), (8, 8));
        assert!(mask[1].iter().any(|&d| d) && mask[1].iter().any(|&d| !d));
        // everything drops: one neuron stays
        let all = Dropout {
            rate: 1.0,
            ..dropout
        }
        .mask(&refs);
        assert_eq!(all[1].iter().filter(|&&d| !d).count(), 1);
        let none = Dropout {
            rate: 0.0,
            ..dropout
        }
        .mask(&refs);
        assert!(none[3].iter().all(|&d| !d));
    }

    #[test]
    fn dropped_neurons_fade_out_and_back() {
        let dropout = Dropout {
            start: 1.0,
            duration: 2.0,
            ..Default::default()
        };
        assert_eq!(dropout.amount(0.5), 0.0);
        assert!((dropout.amount(1.1) - 0.5).abs() < 1e-5);
        assert_eq!(dropout.amount(2.0), 1.0);
        assert!((dropout.amount(2.9) - 0.5).abs() < 1e-5);
        assert_eq!(dropout.amount(3.5), 0.0);
    }

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
