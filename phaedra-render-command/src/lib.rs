use phaedra_color_types::LinearRgba;

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
        zindex: i8,
        rect: RectF,
        color: LinearRgba,
        hsv: Option<HsbTransform>,
    },
    DrawQuad {
        layer: usize,
        zindex: i8,
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
    Nop,
}

impl RenderCommand {
    pub fn and_then<F>(self, f: F) -> RenderCommand
    where
        F: FnOnce(RenderCommand) -> RenderCommand,
    {
        f(self)
    }

    pub fn content_hash(commands: &[Self]) -> u64 {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for cmd in commands {
            cmd.hash_command(&mut hasher);
        }
        hasher.finish()
    }

    fn hash_command(&self, hasher: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        std::mem::discriminant(self).hash(hasher);
        match self {
            Self::Clear { color } => {
                hash_linear_rgba(color, hasher);
            }
            Self::FillRect {
                layer,
                zindex,
                rect,
                color,
                hsv,
            } => {
                layer.hash(hasher);
                zindex.hash(hasher);
                hash_rectf(rect, hasher);
                hash_linear_rgba(color, hasher);
                hash_opt_hsb(hsv, hasher);
            }
            Self::DrawQuad {
                layer,
                zindex,
                position,
                texture,
                fg_color,
                alt_color,
                hsv,
                mode,
            } => {
                layer.hash(hasher);
                zindex.hash(hasher);
                hash_rectf(position, hasher);
                hash_texture_coords(texture, hasher);
                hash_linear_rgba(fg_color, hasher);
                alt_color.is_some().hash(hasher);
                if let Some((c, mix)) = alt_color {
                    hash_linear_rgba(c, hasher);
                    mix.to_bits().hash(hasher);
                }
                hash_opt_hsb(hsv, hasher);
                std::mem::discriminant(mode).hash(hasher);
            }
            Self::Batch(cmds) => {
                for cmd in cmds.iter() {
                    cmd.hash_command(hasher);
                }
            }
            Self::SetClipRect(r) => {
                r.is_some().hash(hasher);
                if let Some(r) = r {
                    hash_rectf(r, hasher);
                }
            }
            Self::BeginPostProcess | Self::Nop => {}
        }
    }

    pub fn map_colors<F>(self, f: &F) -> RenderCommand
    where
        F: Fn(LinearRgba) -> LinearRgba,
    {
        match self {
            RenderCommand::Clear { color } => RenderCommand::Clear { color: f(color) },
            RenderCommand::FillRect {
                layer,
                zindex,
                rect,
                color,
                hsv,
            } => {
                RenderCommand::FillRect {
                    layer,
                    zindex,
                    rect,
                    color: f(color),
                    hsv,
                }
            }
            RenderCommand::DrawQuad {
                layer,
                zindex,
                position,
                texture,
                fg_color,
                alt_color,
                hsv,
                mode,
            } => RenderCommand::DrawQuad {
                layer,
                zindex,
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

    pub fn clip_to_rect(self, clip: &RectF) -> RenderCommand {
        match self {
            RenderCommand::FillRect {
                layer,
                zindex,
                rect,
                color,
                hsv,
            } => match rect.intersection(clip) {
                Some(clipped) => RenderCommand::FillRect {
                    layer,
                    zindex,
                    rect: clipped,
                    color,
                    hsv,
                },
                None => RenderCommand::Nop,
            },
            RenderCommand::DrawQuad {
                layer,
                zindex,
                position,
                texture,
                fg_color,
                alt_color,
                hsv,
                mode,
            } => match position.intersection(clip) {
                Some(clipped_pos) => {
                    let w = position.size.width;
                    let h = position.size.height;
                    if w <= 0.0 || h <= 0.0 {
                        return RenderCommand::Nop;
                    }
                    let t_left = (clipped_pos.origin.x - position.origin.x) / w;
                    let t_right =
                        (clipped_pos.origin.x + clipped_pos.size.width - position.origin.x) / w;
                    let t_top = (clipped_pos.origin.y - position.origin.y) / h;
                    let t_bottom =
                        (clipped_pos.origin.y + clipped_pos.size.height - position.origin.y) / h;

                    let tex_w = texture.right - texture.left;
                    let tex_h = texture.bottom - texture.top;

                    let clipped_texture = TextureCoords {
                        left: texture.left + t_left * tex_w,
                        right: texture.left + t_right * tex_w,
                        top: texture.top + t_top * tex_h,
                        bottom: texture.top + t_bottom * tex_h,
                    };

                    RenderCommand::DrawQuad {
                        layer,
                        zindex,
                        position: clipped_pos,
                        texture: clipped_texture,
                        fg_color,
                        alt_color,
                        hsv,
                        mode,
                    }
                }
                None => RenderCommand::Nop,
            },
            RenderCommand::Batch(cmds) => {
                let clipped: Vec<_> = cmds
                    .into_iter()
                    .map(|c| c.clip_to_rect(clip))
                    .filter(|c| !matches!(c, RenderCommand::Nop))
                    .collect();
                if clipped.is_empty() {
                    RenderCommand::Nop
                } else {
                    RenderCommand::Batch(clipped)
                }
            }
            other => other,
        }
    }

    pub fn fold<T, F>(&self, init: T, f: &F) -> T
    where
        F: Fn(T, &RenderCommand) -> T,
    {
        match self {
            RenderCommand::Batch(cmds) => cmds.iter().fold(init, |acc, cmd| cmd.fold(acc, f)),
            _ => f(init, self),
        }
    }
}

fn hash_linear_rgba(color: &LinearRgba, hasher: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    color.0.to_bits().hash(hasher);
    color.1.to_bits().hash(hasher);
    color.2.to_bits().hash(hasher);
    color.3.to_bits().hash(hasher);
}

fn hash_rectf(rect: &RectF, hasher: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    rect.origin.x.to_bits().hash(hasher);
    rect.origin.y.to_bits().hash(hasher);
    rect.size.width.to_bits().hash(hasher);
    rect.size.height.to_bits().hash(hasher);
}

fn hash_texture_coords(texture: &TextureCoords, hasher: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    texture.left.to_bits().hash(hasher);
    texture.top.to_bits().hash(hasher);
    texture.right.to_bits().hash(hasher);
    texture.bottom.to_bits().hash(hasher);
}

fn hash_opt_hsb(hsv: &Option<HsbTransform>, hasher: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    hsv.is_some().hash(hasher);
    if let Some(hsv) = hsv {
        hsv.hue.to_bits().hash(hasher);
        hsv.saturation.to_bits().hash(hasher);
        hsv.brightness.to_bits().hash(hasher);
    }
}
