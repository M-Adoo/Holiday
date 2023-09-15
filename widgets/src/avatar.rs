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
/// fn_widget! {
///   @ Avatar {
///     @ { Label::new("A") }
///   }
/// };
///
/// # #[cfg(feature="png")]
/// fn_widget! {
///   @ Avatar {
///     @ { ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png")) }
///   }
/// };
/// ```
#[derive(Declare2, Default, Clone)]
pub struct Avatar {
  #[declare(default=Palette::of(ctx!()).primary())]
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
    fn_widget! {
      @ {
        let AvatarStyle {
          size, radius, text_style,
        } = AvatarStyle::of(ctx!());
        let palette1 = Palette::of(ctx!()).clone();
        let palette2 = Palette::of(ctx!()).clone();
        let w: Widget = match child {
          AvatarTemplate::Text(mut text) => {
            @Container {
              size,
              border_radius: radius.map(Radius::all),
              background: pipe!(Brush::from(palette1.base_of(&$this.color))),
              @Text {
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                text: $text.0.clone(),
                text_style,
                foreground: pipe!(Brush::from(palette2.on_of(&palette2.base_of(&$this.color)))),
              }
            }.into()
          },
          AvatarTemplate::Image(image) => {
            let clip = radius.map(|radius| {
              let path = Path::rect_round(
                &Rect::from_size(size),
                &Radius::all(radius),
              );
              Clip { clip: ClipType::Path(path) }
            });
            @$clip {
              @Container {
                size,
                @$image {
                  box_fit: BoxFit::Contain,
                }
              }
            }.into()
          }
        };

        @SizedBox {
          size,
          @ { w }
        }
      }
    }
    .into()
  }
}
