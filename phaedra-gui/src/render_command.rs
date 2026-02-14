use ::window::color::LinearRgba;

pub type RectF = euclid::default::Rect<f32>;
pub type PointF = euclid::default::Point2D<f32>;

#[derive(Debug, Clone)]
pub enum QuadMode {
    Glyph,
    ColorEmoji,
    BackgroundImage,
    SolidColor,
    GrayScale,
}

#[derive(Debug, Clone)]
pub struct HsbTransform {
    pub hue: f32,
    pub saturation: f32,
    pub brightness: f32,
}

#[derive(Debug, Clone)]
pub struct TextureCoords {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone)]
pub enum RenderCommand {
    Clear {
        color: LinearRgba,
    },
    FillRect {
        layer: usize,
        rect: RectF,
        color: LinearRgba,
    },
    DrawQuad {
        layer: usize,
        position: RectF,
        texture: TextureCoords,
        fg_color: LinearRgba,
        alt_color: Option<(LinearRgba, f32)>,
        hsv: Option<HsbTransform>,
        mode: QuadMode,
    },
    SetClipRect(Option<RectF>),
    BeginPostProcess,
    Batch(Vec<RenderCommand>),
}

impl RenderCommand {
    pub fn and_then<F>(self, f: F) -> RenderCommand
    where
        F: FnOnce(RenderCommand) -> RenderCommand,
    {
        f(self)
    }

    pub fn map_colors<F>(self, f: &F) -> RenderCommand
    where
        F: Fn(LinearRgba) -> LinearRgba,
    {
        match self {
            RenderCommand::Clear { color } => RenderCommand::Clear { color: f(color) },
            RenderCommand::FillRect { layer, rect, color } => {
                RenderCommand::FillRect {
                    layer,
                    rect,
                    color: f(color),
                }
            }
            RenderCommand::DrawQuad {
                layer,
                position,
                texture,
                fg_color,
                alt_color,
                hsv,
                mode,
            } => RenderCommand::DrawQuad {
                layer,
                position,
                texture,
                fg_color: f(fg_color),
                alt_color: alt_color.map(|(c, mix)| (f(c), mix)),
                hsv,
                mode,
            },
            RenderCommand::Batch(cmds) => {
                RenderCommand::Batch(cmds.into_iter().map(|c| c.map_colors(f)).collect())
            }
            other => other,
        }
    }

    pub fn fold<T, F>(&self, init: T, f: &F) -> T
    where
        F: Fn(T, &RenderCommand) -> T,
    {
        match self {
            RenderCommand::Batch(cmds) => {
                cmds.iter().fold(f(init, self), |acc, cmd| cmd.fold(acc, f))
            }
            _ => f(init, self),
        }
    }
}
