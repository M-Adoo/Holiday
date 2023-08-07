use crate::prelude::*;
use ribir_core::prelude::*;

/// Avatar usage
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Avatar {
///     Label::new("A")
///   }
/// };
///
/// # #[cfg(feature="png")]
/// widget! {
///   Avatar {
///     ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png"))
///   }
/// };
/// ```
#[derive(Declare, Declare2, Default, Clone)]
pub struct Avatar {
  #[declare(default=Palette::of(ctx).primary())]
  pub color: Color,
}

#[derive(Clone)]
pub struct AvatarStyle {
  pub size: Size,
  pub radius: Option<f32>,
  pub text_style: CowArc<TextStyle>,
}

impl CustomStyle for AvatarStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    AvatarStyle {
      size: Size::splat(40.),
      radius: Some(20.),
      text_style: TypographyTheme::of(ctx).body_large.text.clone(),
    }
  }
}

pub struct AvatarDecorator;

impl ComposeDecorator for AvatarDecorator {
  type Host = Widget;

  fn compose_decorator(_: State<Self>, host: Self::Host) -> Widget { host }
}

#[derive(Template)]
pub enum AvatarTemplate {
  Text(State<Label>),
  Image(ShareResource<PixelImage>),
}

impl ComposeChild for Avatar {
  type Child = AvatarTemplate;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let AvatarStyle {
          size, radius, text_style,
        } = AvatarStyle::of(ctx).clone();
        let palette1 = Palette::of(ctx).clone();
        let palette2 = Palette::of(ctx).clone();
      }
      SizedBox {
        size,
        widget::from::<Widget>(match child {
          AvatarTemplate::Text(text) => widget! {
            states { text: text.into_readonly() }
            BoxDecoration {
              background: Brush::from(palette1.base_of(&this.color)),
              border_radius: radius.map(Radius::all),
              Container {
                size,
                Text {
                  h_align: HAlign::Center,
                  v_align: VAlign::Center,
                  text: text.0.clone(),
                  text_style: text_style.clone(),
                  foreground: Brush::from(palette2.on_of(&palette2.base_of(&this.color))),
                }
              }
            }
          }.into(),
          AvatarTemplate::Image(image) => widget! {
            DynWidget {
              dyns: radius.map(|radius| {
                let path = Path::rect_round(
                  &Rect::from_size(size),
                  &Radius::all(radius),
                );
                Clip { clip: ClipType::Path(path) }
              }),
              Container {
                size,
                DynWidget{
                  box_fit: BoxFit::Contain,
                  dyns: image,
                }
              }
            }
          }.into()
        })
      }
    }
    .into()
  }
}
