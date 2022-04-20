//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used as an attribute to attach to a widget, query
//! theme data from `BuildCtx`. Use `Theme` widgets to specify part of
//! application's theme. Application theme is use `Theme` widget as root of all
//! windows.
pub mod material;
pub use painter::*;
use text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

#[derive(Clone, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}

bitflags! {
  /// A linear decoration to draw near the text.
  #[derive(Default)]
  pub struct  TextDecoration: u8 {
    const NONE = 0b0001;
    /// Draw a line underneath each line of text
    const UNDERLINE =  0b0010;
    /// Draw a line above each line of text
    const OVERLINE = 0b0100;
    /// Draw a line through each line of text
    const THROUGHLINE = 0b1000;
  }
}

/// Encapsulates the text decoration style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecorationStyle {
  /// The decorations to paint near the text
  pub decoration: TextDecoration,
  /// The color in which to paint the text decorations.
  pub decoration_color: Brush,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextTheme {
  pub text: TextStyle,
  pub decoration: TextDecorationStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBoxDecorationStyle {
  pub background: Brush,

  /// The corners of this box are rounded by this `BorderRadius`. The round
  /// corner only work if the two borders beside it are same style.]
  pub radius: Option<Radius>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBarTheme {
  pub track_box: ScrollBoxDecorationStyle,
  pub track_width: f32,

  pub thumb_box: ScrollBoxDecorationStyle,
  pub thumb_width: f32,
}

/// Use typography to present your design and content as clearly and efficiently
/// as possible. The names of the TextTheme properties from the [Material Design
/// spec](https://material.io/design/typography/the-type-system.html#applying-the-type-scale)
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTheme {
  pub headline1: TextTheme,
  pub headline2: TextTheme,
  pub headline3: TextTheme,
  pub headline4: TextTheme,
  pub headline5: TextTheme,
  pub headline6: TextTheme,
  pub subtitle1: TextTheme,
  pub subtitle2: TextTheme,
  pub body1: TextTheme,
  pub body2: TextTheme,
  pub button: TextTheme,
  pub caption: TextTheme,
  pub overline: TextTheme,
}

// todo: theme can provide fonts folder.

/// Properties from [Material Theme](https://material.io/design/material-theming/implementing-your-theme.html)
#[derive(Clone, Debug)]
pub struct Theme {
  // Dark or light theme.
  pub brightness: Brightness,
  pub primary: Color,
  pub primary_variant: Color,
  pub secondary: Color,
  pub secondary_variant: Color,
  pub background: Color,
  pub surface: Color,
  pub error: Color,
  pub on_primary: Color,
  pub on_secondary: Color,
  pub on_background: Color,
  pub on_surface: Color,
  pub on_error: Color,
  pub typography_theme: TypographyTheme,
  /// The color used for widgets in their inactive (but enabled) state.
  pub unselected_widget_color: Color,
  /// Default text font families
  pub default_font_family: Box<[FontFamily]>,
  pub checkbox: CheckboxTheme,
  pub scrollbar: ScrollBarTheme,
  pub icon: IconTheme,
}

impl TypographyTheme {
  /// Create a TypographyTheme which implement the typography styles base on the
  /// material design specification.
  ///
  /// The `titles_family` applied to headlines and subtitles and `body_family`
  /// applied to body and caption. The `display_style` is applied to
  /// headline4, headline3, headline2, headline1, and caption. The
  /// `body_style` is applied to the remaining text styles.
  pub fn new(
    titles_family: Box<[FontFamily]>,
    body_family: Box<[FontFamily]>,
    display_style: Brush,
    body_style: Brush,
    decoration: TextDecoration,
    decoration_color: Brush,
  ) -> Self {
    let decoration = TextDecorationStyle { decoration, decoration_color };
    let light_title_face = FontFace {
      families: titles_family,
      weight: FontWeight::LIGHT,
      ..<_>::default()
    };

    let mut normal_title_face = light_title_face.clone();
    normal_title_face.weight = FontWeight::NORMAL;

    let mut medium_title_face = light_title_face.clone();
    medium_title_face.weight = FontWeight::MEDIUM;

    let body_face = FontFace {
      families: body_family,
      ..<_>::default()
    };

    Self {
      headline1: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(96.0.into()),
          letter_space: Some(Pixel::from(-1.5)),
          foreground: display_style.clone(),
          font_face: light_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline2: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(60.0.into()),
          letter_space: Some(Pixel::from(-0.5)),
          foreground: display_style.clone(),
          font_face: light_title_face,
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline3: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(48.0.into()),
          foreground: display_style.clone(),
          letter_space: Some(Pixel(0.0.into())),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },

      headline4: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(34.0.into()),
          foreground: display_style.clone(),
          letter_space: Some(Pixel(0.25.into())),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline5: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(24.0.into()),
          letter_space: Some(Pixel(0.0.into())),
          foreground: body_style.clone(),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline6: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(20.0.into()),
          letter_space: Some(Pixel(0.15.into())),
          foreground: body_style.clone(),
          font_face: medium_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },

      subtitle1: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(16.0.into()),
          letter_space: Some(Pixel(0.15.into())),
          foreground: body_style.clone(),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      subtitle2: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(14.0.into()),
          letter_space: Some(Pixel(0.1.into())),
          foreground: body_style.clone(),
          font_face: medium_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      body1: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(16.0.into()),
          letter_space: Some(Pixel(0.5.into())),
          foreground: body_style.clone(),
          font_face: body_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },

      body2: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(14.0.into()),
          letter_space: Some(Pixel(0.25.into())),
          foreground: body_style.clone(),
          font_face: body_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      button: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(14.0.into()),
          letter_space: Some(Pixel(1.25.into())),
          foreground: body_style.clone(),
          font_face: {
            let mut face = body_face.clone();
            face.weight = FontWeight::MEDIUM;
            face
          },
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      caption: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(12.0.into()),
          letter_space: Some(Pixel(0.4.into())),
          foreground: body_style.clone(),
          font_face: body_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      overline: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(10.0.into()),
          letter_space: Some(Pixel(1.5.into())),
          foreground: body_style,
          font_face: body_face,
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration,
      },
    }
  }
}

#[derive(Debug, Clone)]
pub struct CheckboxTheme {
  pub size: f32,
  pub check_background: Color,
  // todo: use border merge border_width & border_color ~
  pub border_width: f32,
  pub radius: f32,
  pub border_color: Color,
  pub checked_path: Path,
  pub indeterminate_path: Path,
}

impl Default for CheckboxTheme {
  fn default() -> Self {
    let size: f32 = 12.;
    let border_width = 2.;
    let checked_path = {
      let mut builder = Path::builder();
      let start = Point::new(2.733_333_3, 8.466_667);
      let mid = Point::new(6., 11.733_333);
      let end = Point::new(13.533_333, 4.2);
      builder.segment(start, mid).segment(mid, end);
      builder.stroke(1.422_222, Color::WHITE.into())
    };

    let center_y = size / 2. + border_width;
    let indeterminate_path = {
      let mut builder = Path::builder();
      builder
        .begin_path(Point::new(3., center_y))
        .line_to(Point::new(size + border_width * 2. - 3., center_y))
        .close_path();
      builder.stroke(border_width, Color::WHITE.into())
    };

    Self {
      size,
      border_width,
      check_background: Color::BLACK,
      radius: 2.,
      border_color: Color::BLACK,
      checked_path,
      indeterminate_path,
    }
  }
}

#[derive(Debug, Clone)]
pub struct IconTheme {
  pub width: f32,
  pub height: f32,
  pub fill_color: Color,
  pub stroke_color: Color,
}

impl Default for IconTheme {
  fn default() -> Self {
    Self {
      width: 16.0,
      height: 16.0,
      fill_color: Color::WHITE,
      stroke_color: Color::BLACK,
    }
  }
}
