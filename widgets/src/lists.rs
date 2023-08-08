use crate::prelude::*;
use ribir_core::prelude::*;

/// Lists usage
///
/// use `ListItem` must have `HeadlineText`, other like `SupportingText`,
/// `Leading`, and `Trailing` are optional.
///
/// # Example
///
/// ## single headline text
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // only single headline text
/// widget! {
///   Lists {
///     ListItem {
///       HeadlineText(Label::new("One line list item"))
///     }
///   }
/// };
/// ```
///
/// ## headline text and supporting text
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // single headline text and supporting text
/// widget! {
///   Lists {
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       SupportingText(Label::new("supporting text"))
///     }
///   }
/// };
/// ```
///
/// ## use leading
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Lists {
///     // use leading icon
///     ListItem {
///       Leading { svgs::CHECK_BOX_OUTLINE_BLANK }
///       HeadlineText(Label::new("headline text"))
///     }
///     // use leading label
///     ListItem {
///       Leading { Label::new("A") }
///       HeadlineText(Label::new("headline text"))
///     }
///     // use leading custom widget
///     ListItem {
///       Leading {
///         CustomEdgeWidget(widget! {
///           Container {
///             size: Size::splat(40.),
///             background: Color::YELLOW,
///           }
///         }.into())
///       }
///     }
///   }
/// };
/// ```
///
/// ## use trailing
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Lists {
///     // use trailing icon
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       Trailing { svgs::CHECK_BOX_OUTLINE_BLANK }
///     }
///     // use trailing label
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       Trailing { Label::new("A") }
///     }
///     // use trailing custom widget
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       Trailing {
///         CustomEdgeWidget(widget! {
///           Container {
///             size: Size::splat(40.),
///             background: Color::YELLOW,
///           }
///         }.into())
///       }
///     }
///   }
/// };
/// ```
///
/// ## use `Divider` split list item
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Lists {
///     ListItem {
///       HeadlineText(Label::new("One line list item"))
///     }
///     Divider {}
///     ListItem {
///       HeadlineText(Label::new("One line list item"))
///     }
///   }
/// };
/// ```
#[derive(Declare, Declare2)]
pub struct Lists;

#[derive(Declare)]
pub struct ListsDecorator {}
impl ComposeDecorator for ListsDecorator {
  type Host = Widget;

  fn compose_decorator(_: State<Self>, host: Self::Host) -> Widget { host }
}

impl ComposeChild for Lists {
  type Child = Vec<Widget>;

  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    widget! {
      ListsDecorator {
        Column {
          Multi::new(child)
        }
      }
    }
    .into()
  }
}

#[derive(Clone, Default)]
pub struct EdgeItemStyle {
  pub size: Size,
  pub gap: Option<EdgeInsets>,
}

#[derive(Clone)]
pub struct EdgeTextItemStyle {
  pub style: CowArc<TextStyle>,
  pub gap: Option<EdgeInsets>,
  pub foreground: Brush,
}

#[derive(Clone)]
pub struct EdgeWidgetStyle {
  pub icon: EdgeItemStyle,
  pub text: EdgeTextItemStyle,
  pub avatar: EdgeItemStyle,
  pub image: EdgeItemStyle,
  pub poster: EdgeItemStyle,
  pub custom: EdgeItemStyle,
}

pub struct Poster(pub ShareResource<PixelImage>);

pub struct HeadlineText(pub Label);
pub struct SupportingText(pub Label);

#[derive(Template)]
pub enum EdgeWidget {
  Text(State<Label>),
  Icon(NamedSvg),
  Avatar(ComposePair<State<Avatar>, AvatarTemplate>),
  Image(ShareResource<PixelImage>),
  Poster(Poster),
  Custom(CustomEdgeWidget),
}

pub struct CustomEdgeWidget(pub Widget);

impl EdgeWidget {
  fn compose_with_style(self, config: EdgeWidgetStyle) -> Widget {
    let EdgeWidgetStyle {
      icon,
      text,
      avatar,
      image,
      poster,
      custom,
    } = config;
    match self {
      EdgeWidget::Icon(w) => widget! {
        DynWidget {
          dyns: icon.gap.map(|margin| Margin { margin }),
          Icon {
            size: icon.size,
            widget::from(w)
          }
        }
      }
      .into(),
      EdgeWidget::Text(w) => widget! {
        DynWidget {
          dyns: text.gap.map(|margin| Margin { margin }),
          widget!{
            states { label: w.into_readonly() }
            Text {
              text: label.0.clone(),
              text_style: text.style.clone(),
              foreground: text.foreground.clone(),
            }
          }
        }
      }
      .into(),
      EdgeWidget::Avatar(w) => widget! {
        DynWidget {
          dyns: avatar.gap.map(|margin| Margin { margin }),
          widget::from(w)
        }
      }
      .into(),
      EdgeWidget::Image(w) => widget! {
        DynWidget {
          dyns: image.gap.map(|margin| Margin { margin }),
          SizedBox {
            size: image.size,
            DynWidget {
              box_fit: BoxFit::None,
              dyns: w
            }
          }
        }
      }
      .into(),
      EdgeWidget::Poster(w) => widget! {
        DynWidget {
          dyns: poster.gap.map(|margin| Margin { margin }),
          SizedBox {
            size: poster.size,
            DynWidget {
              box_fit: BoxFit::None,
              dyns: w.0
            }
          }
        }
      }
      .into(),
      EdgeWidget::Custom(w) => widget! {
        DynWidget {
          dyns: custom.gap.map(|margin| Margin { margin }),
          widget::from(w.0)
        }
      }
      .into(),
    }
  }
}

#[derive(Template)]
pub struct ListItemTml {
  headline: State<HeadlineText>,
  supporting: Option<State<SupportingText>>,
  leading: Option<FatObj<SinglePair<Leading, EdgeWidget>>>,
  trailing: Option<FatObj<SinglePair<Trailing, EdgeWidget>>>,
}

impl ComposeChild for ListItem {
  type Child = ListItemTml;

  fn compose_child(mut this: State<Self>, child: Self::Child) -> Widget {
    let ListItemTml {
      mut headline,
      supporting,
      leading,
      trailing,
    } = child;

    fn_widget! {
      let palette = Palette::of(ctx!());
      let ListItemStyle {
        padding_style,
        label_gap,
        headline_style,
        supporting_style,
        leading_config,
        trailing_config,
        item_align,
      } = ListItemStyle::of(ctx).clone();

      let padding = padding_style.map(|padding| Padding { padding });
      let label_gap = label_gap.map(|padding| Padding { padding });

      @ListItemDecorator {
        color: pipe!($this.active_background),
        is_active: false,
        @ $padding {
          @Row {
            align_items: pipe!(item_align($this.line_number)),
            @{
              leading.map(|w| {
                let (SinglePair { child,.. }, builtin) = w.unzip();
                builtin.compose_with_host(child.compose_with_style(leading_config), ctx!())
              })
            }
            @Expanded {
              flex: 1.,
              @ $label_gap {
                @Column {
                  @Text {
                    text: pipe!($headline.0.0.clone()),
                    foreground: palette.on_surface().clone(),
                    text_style: headline_style,
                  }
                  @{ supporting.map(|mut supporting|  {
                    @ConstrainedBox {
                      clamp: {
                        let TextStyle { line_height, font_size, .. } = &*supporting_style;
                        let line_height = line_height.map_or(*font_size, FontSize::Em).into_pixel();
                        pipe!{
                          let text_height = line_height * $this.line_number as f32;
                          BoxClamp::fixed_height(*text_height.0)
                        }
                      } ,
                      @Text {
                        text: pipe!($supporting.0.0.clone()),
                        foreground:  palette.on_surface_variant().clone(),
                        text_style: supporting_style,
                      }
                    }
                  })}
                }
              }
            }
            @{
              trailing.map(|w| {
                let (SinglePair { child,.. }, builtin) = w.unzip();
                builtin.compose_with_host(child.compose_with_style(trailing_config), ctx!())
              })
            }
          }
        }
      }
    }
    .into()
  }
}

#[derive(Declare, Declare2)]
pub struct ListItem {
  #[declare(default = 1usize)]
  pub line_number: usize,
  #[declare(default = Palette::of(ctx).primary())]
  pub active_background: Color,
}

#[derive(Clone)]
pub struct ListItemStyle {
  pub padding_style: Option<EdgeInsets>,
  pub label_gap: Option<EdgeInsets>,
  pub item_align: fn(usize) -> Align,
  pub headline_style: CowArc<TextStyle>,
  pub supporting_style: CowArc<TextStyle>,
  pub leading_config: EdgeWidgetStyle,
  pub trailing_config: EdgeWidgetStyle,
}

impl CustomStyle for ListItemStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    let typography = TypographyTheme::of(ctx);
    let palette = Palette::of(ctx);
    ListItemStyle {
      padding_style: Some(EdgeInsets {
        left: 0.,
        right: 24.,
        bottom: 8.,
        top: 8.,
      }),
      item_align: |num| {
        if num >= 2 {
          Align::Start
        } else {
          Align::Center
        }
      },
      label_gap: Some(EdgeInsets::only_left(16.)),
      headline_style: typography.body_large.text.clone(),
      supporting_style: typography.body_medium.text.clone(),
      leading_config: EdgeWidgetStyle {
        icon: EdgeItemStyle {
          size: Size::splat(24.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        text: EdgeTextItemStyle {
          style: typography.label_small.text.clone(),
          foreground: palette.on_surface_variant().into(),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        avatar: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        image: EdgeItemStyle {
          size: Size::splat(56.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        poster: EdgeItemStyle {
          size: Size::new(120., 64.),
          gap: None,
        },
        custom: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
      },
      trailing_config: EdgeWidgetStyle {
        icon: EdgeItemStyle {
          size: Size::splat(24.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        text: EdgeTextItemStyle {
          style: typography.label_small.text.clone(),
          foreground: palette.on_surface_variant().into(),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        avatar: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        image: EdgeItemStyle {
          size: Size::splat(56.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        poster: EdgeItemStyle {
          size: Size::new(120., 64.),
          gap: None,
        },
        custom: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
      },
    }
  }
}

#[derive(Clone, Declare, Declare2)]
pub struct ListItemDecorator {
  pub color: Color,
  pub is_active: bool,
}

impl ComposeDecorator for ListItemDecorator {
  type Host = Widget;
  fn compose_decorator(_: State<Self>, host: Self::Host) -> Widget { host }
}
