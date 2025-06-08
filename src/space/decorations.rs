//! Decorations rendering.
//!
//! This is achieved using a GlesPixelShader, nothing special otherwise.

use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::Uniform;
use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::renderer::shaders::{ShaderElement, Shaders};
use crate::renderer::AsGlowRenderer;

pub fn draw_border(
    renderer: &mut impl AsGlowRenderer,
    scale: i32,
    alpha: f32,
    geometry: Rectangle<i32, Logical>,
    thickness: f64,
    radius: f64,
    color: fht_compositor_config::Color,
) -> ShaderElement {
    let scaled_thickness = thickness * scale as f64;
    let (start_color, end_color, angle) = match color {
        fht_compositor_config::Color::Solid(color) => (color, color, 0.0),
        fht_compositor_config::Color::Gradient { start, end, angle } => (start, end, angle),
    };

    ShaderElement::new(
        Shaders::get(renderer.glow_renderer()).border.clone(),
        geometry,
        None,
        alpha,
        vec![
            Uniform::new("v_start_color", start_color),
            Uniform::new("v_end_color", end_color),
            Uniform::new("v_gradient_angle", angle),
            // NOTE: For some reasons we cant use f64s, we shall cast
            Uniform::new("thickness", scaled_thickness as f32),
            Uniform::new("corner_radius", radius as f32),
        ],
        Kind::Unspecified,
    )
}

/// A drop shadow around a tile.
///
/// The shadow uses a [`FhtPixelShaderElement`] under the hood to do the drawing. The given
/// rectangle will receive a drop shadow *around it*, so it doesn't account for the actual shadow
/// geometry
#[derive(Debug)]
pub struct Shadow {
    /// The underlying [`PixelShaderElement`] that draws the border.
    // Use a OnceCell to not require a `impl FhtRenderer` for creating Tiles
    element: OnceCell<FhtPixelShaderElement>,
    /// The rectangle we want to draw a shadow around.
    rectangle: Rectangle<i32, Logical>,
    /// The configuration of the shadow.
    config: fht_compositor_config::Shadow,
    /// The corner radius of the shadow.
    corner_radius: f32,
}

impl Shadow {
    /// Create a new [`Shadow`] for a tile.
    pub fn new(
        rectangle: Rectangle<i32, Logical>,
        corner_radius: f32,
        config: fht_compositor_config::Shadow,
    ) -> Self {
        Self {
            element: OnceCell::new(),
            rectangle,
            config,
            corner_radius,
        }
    }

    /// Set the rectangle to draw the shadow around.
    pub fn set_rectangle(&mut self, rectangle: Rectangle<i32, Logical>) {
        self.rectangle = rectangle;
        if let Some(element) = self.element.get_mut() {
            let mut element_geometry = rectangle;
            let sigma = self.config.sigma.ceil() as i32;
            element_geometry.loc -= Point::from((sigma, sigma));
            element_geometry.size += Size::from((sigma, sigma)).upscale(2);

            element.resize(element_geometry, None);
        }
    }

    /// Update the parameters of this [`Shadow`]
    pub fn update_parameters(&mut self, config: fht_compositor_config::Shadow, corner_radius: f32) {
        if self.config != config || self.corner_radius != corner_radius {
            self.corner_radius = corner_radius;
            self.config = config;
            self.update_uniforms();
        }
    }

    /// Generate uniform values from the current state of this [`Shadow`]
    fn get_uniforms(&self) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("shadow_color", self.config.color),
            Uniform::new("blur_sigma", self.config.sigma),
            Uniform::new("corner_radius", self.corner_radius),
        ]
    }

    /// Update the uniform values passed into the [`FhtPixelShaderElement`].
    fn update_uniforms(&mut self) {
        let uniforms = self.get_uniforms();
        if let Some(element) = self.element.get_mut() {
            element.update_uniforms(uniforms);
        }
    }

    /// Get a render element for this [`Shadow`]
    pub fn render(
        &self,
        renderer: &mut impl FhtRenderer,
        is_floating: bool,
    ) -> Option<FhtPixelShaderElement> {
        if self.config.disable || self.config.floating_only && !is_floating {
            return None;
        }

        let element = self
            .element
            .get_or_init(|| {
                let program = Shaders::get(renderer.glow_renderer()).box_shadow.clone();
                let uniforms = self.get_uniforms();
                let mut element_geometry = self.rectangle;
                let sigma = self.config.sigma.ceil() as i32;
                element_geometry.loc -= Point::from((sigma, sigma));
                element_geometry.size += Size::from((sigma, sigma)).upscale(2);

                FhtPixelShaderElement::new(
                    program,
                    element_geometry,
                    1.0,
                    uniforms,
                    Kind::Unspecified,
                )
            })
            .clone();
        Some(element)
    }
}

// Shadow drawing shader using the following article code:
// https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/
pub fn draw_shadow(
    renderer: &mut impl AsGlowRenderer,
    alpha: f32,
    scale: i32,
    mut geometry: Rectangle<i32, Logical>,
    blur_sigma: f32,
    corner_radius: f32,
    color: [f32; 4],
) -> ShaderElement {
    let scaled_blur_sigma = (blur_sigma / scale as f32).round() as i32;
    geometry.loc -= Point::from((scaled_blur_sigma, scaled_blur_sigma));
    geometry.size += Size::from((2 * scaled_blur_sigma, 2 * scaled_blur_sigma));

    ShaderElement::new(
        Shaders::get(renderer.glow_renderer()).box_shadow.clone(),
        geometry,
        None,
        alpha,
        vec![
            // NOTE: For some reasons we cant use f64s, we shall cast
            Uniform::new("shadow_color", color),
            Uniform::new("blur_sigma", blur_sigma),
            Uniform::new("corner_radius", corner_radius),
        ],
        Kind::Unspecified,
    )
}
