use ribir::prelude::*;
use ribir_material::material_svgs;
use std::rc::Rc;

const NORMAL_BUTTON_SIZE: Size = Size::new(120., 40.);

struct App;

impl Compose for App {
  fn compose(_: State<Self>) -> Widget {
    widget! {
      init ctx => {
        let surface_container_low: Brush = Palette::of(ctx).surface_container_low().clone().into();
        let primary: Brush = Palette::of(ctx).primary().clone().into();
        let surface_container_lowest: Brush = Palette::of(ctx)
          .surface_container_lowest().clone().into();
      }
      ThemeWidget {
        id: theme,
        theme: Rc::new(Theme::Full(ribir_material::purple::light())),
        Column {
          background: surface_container_low.clone(),
          ConstrainedBox {
            clamp: BoxClamp::fixed_height(48.),
            Row {
              v_align: VAlign::Center,
              justify_content: JustifyContent::SpaceAround,
              Text {
                text: "Material 3",
              }
              Checkbox {
                id: brightness,
                on_tap: move |_| {
                  if brightness.checked {
                    theme.theme = Rc::new(Theme::Full(ribir_material::purple::dark()));
                  } else {
                    theme.theme = Rc::new(Theme::Full(ribir_material::purple::light()));
                  }
                },
                Trailing {
                  Label::new("Dark")
                }
              }
            }
          }
          Expanded {
            flex: 1.,
            Tabs {
              pos: Position::Bottom,
              Tab {
                TabItem {
                  Label::new("Actions")
                }
                TabPane {
                  VScrollBar {
                    Column {
                      Column {
                        align_items: Align::Center,
                        ConstrainedBox {
                          clamp: BoxClamp::fixed_height(30.),
                          Row {
                            h_align: HAlign::Center,
                            Text { text: "Common buttons" }
                            Icon {
                              size: Size::splat(16.),
                              material_svgs::INFO
                            }
                          }
                        }
                        Column {
                          item_gap: 20.,
                          padding: EdgeInsets::new(20., 40., 20., 40.),
                          background: surface_container_lowest.clone(),
                          border_radius: Radius::all(4.),
                          border: Border::all(BorderSide {
                            color: primary.clone(),
                            width: 1.,
                          }),
                          Row {
                            item_gap: 20.,
                            SizedBox {
                              size: NORMAL_BUTTON_SIZE,
                              FilledButton {
                                Label::new("Filled")
                              }
                            }
                            SizedBox {
                              size: NORMAL_BUTTON_SIZE,
                              FilledButton {
                                svgs::ADD
                                Label::new("Icon")
                              }
                            }
                          }
                          Row {
                            item_gap: 20.,
                            SizedBox {
                              size: NORMAL_BUTTON_SIZE,
                              OutlinedButton {
                                Label::new("Outlined")
                              }
                            }
                            SizedBox {
                              size: NORMAL_BUTTON_SIZE,
                              OutlinedButton {
                                svgs::ADD
                                Label::new("Icon")
                              }
                            }
                          }
                          Row {
                            item_gap: 20.,
                            SizedBox {
                              size: NORMAL_BUTTON_SIZE,
                              Button {
                                Label::new("Text")
                              }
                            }
                            SizedBox {
                              size: NORMAL_BUTTON_SIZE,
                              Button {
                                svgs::ADD
                                Label::new("Icon")
                              }
                            }
                          }
                        }
                      }
                      Column {
                        align_items: Align::Center,
                        ConstrainedBox {
                          clamp: BoxClamp::fixed_height(30.),
                          Row {
                            h_align: HAlign::Center,
                            Text { text: "Floating action buttons" }
                            Icon {
                              size: Size::splat(16.),
                              material_svgs::INFO
                            }
                          }
                        }
                        Column {
                          item_gap: 20.,
                          padding: EdgeInsets::new(20., 40., 20., 40.),
                          background: surface_container_lowest.clone(),
                          border_radius: Radius::all(4.),
                          border: Border::all(BorderSide {
                            color: primary.clone(),
                            width: 1.,
                          }),
                          Row {
                            item_gap: 20.,
                            FabButton {
                              svgs::ADD
                            }
                            FabButton {
                              svgs::ADD
                              Label::new("Create")
                            }
                          }
                        }
                      }
                      Column {
                        align_items: Align::Center,
                        ConstrainedBox {
                          clamp: BoxClamp::fixed_height(30.),
                          Row {
                            h_align: HAlign::Center,
                            Text { text: "Icon buttons" }
                            Icon {
                              size: Size::splat(16.),
                              material_svgs::INFO
                            }
                          }
                        }
                        Column {
                          item_gap: 20.,
                          padding: EdgeInsets::new(20., 40., 20., 40.),
                          background: surface_container_lowest.clone(),
                          border_radius: Radius::all(4.),
                          border: Border::all(BorderSide {
                            color: primary.clone(),
                            width: 1.,
                          }),
                          Row {
                            item_gap: 20.,
                            Button {
                              svgs::SETTINGS
                            }
                            FilledButton {
                              svgs::SETTINGS
                            }
                            OutlinedButton {
                              svgs::SETTINGS
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
              Tab {
                TabItem {
                  Label::new("Tabs")
                }
                TabPane {
                  Tabs {
                    Tab {
                      TabItem {
                        Label::new("Video")
                      }
                    }
                    Tab {
                      TabItem {
                        Label::new("Photos")
                      }
                    }
                    Tab {
                      TabItem {
                        Label::new("Audio")
                      }
                    }
                  }
                }
              }
              Tab {
                TabItem {
                  Label::new("Containment")
                }
                TabPane {
                  Column {
                    ConstrainedBox {
                      clamp: BoxClamp::fixed_height(30.),
                      Row {
                        h_align: HAlign::Center,
                        Text { text: "Divider" }
                        Icon {
                          size: Size::splat(16.),
                          material_svgs::INFO
                        }
                      }
                    }
                    Divider {}
                  }
                }
              }
              Tab {
                TabItem {
                  Label::new("Selections")
                }
                TabPane {
                  Column {
                    Checkbox {
                      Leading {
                        Label::new("Option1")
                      }
                    }
                    Checkbox {
                      Leading {
                        Label::new("Option2")
                      }
                    }
                    Checkbox {
                      Leading {
                        Label::new("Option3")
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

fn main() {
  let system_theme = ribir_material::purple::light();
  let app: Application = Application::new(system_theme);
  let root = App {};
  let wnd = Window::builder(root.into_widget())
    .with_inner_size(Size::new(1024., 768.))
    .with_title("Material 3 Theme Show Case")
    .build(&app);

  ribir::prelude::app::run_with_window(app, wnd);
}
