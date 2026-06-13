use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default)]
pub struct VideoPictureTune {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub hue: f32,
}

impl VideoPictureTune {
    pub fn from_ui_values(
        brightness: i32,
        contrast: i32,
        saturation: i32,
        gamma: i32,
        hue: i32,
    ) -> Self {
        Self {
            brightness: normalize_ui_value(brightness),
            contrast: normalize_ui_value(contrast),
            saturation: normalize_ui_value(saturation),
            gamma: normalize_ui_value(gamma),
            hue: normalize_ui_value(hue),
        }
    }

    pub fn clamped(self) -> Self {
        Self {
            brightness: clamp_unit(self.brightness),
            contrast: clamp_unit(self.contrast),
            saturation: clamp_unit(self.saturation),
            gamma: clamp_unit(self.gamma),
            hue: clamp_unit(self.hue),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PictureTuneShaderValues {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub hue: f32,
}

pub(super) fn picture_tune_shader_values(tune: VideoPictureTune) -> PictureTuneShaderValues {
    let tune = tune.clamped();
    PictureTuneShaderValues {
        brightness: tune.brightness * 0.12,
        contrast: tune.contrast * 0.25,
        saturation: tune.saturation * 0.45,
        gamma: tune.gamma * 0.35,
        hue: tune.hue * PI,
    }
}

fn normalize_ui_value(value: i32) -> f32 {
    (value as f32 / 100.0).clamp(-1.0, 1.0)
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_zero_maps_to_neutral_shader_values() {
        let shader = picture_tune_shader_values(VideoPictureTune::default());
        assert_eq!(shader.brightness, 0.0);
        assert_eq!(shader.contrast, 0.0);
        assert_eq!(shader.saturation, 0.0);
        assert_eq!(shader.gamma, 0.0);
        assert_eq!(shader.hue, 0.0);
    }
}
