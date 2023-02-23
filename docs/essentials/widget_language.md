---
sidebar_position: 1
---

# The `widget!` macro

The `widget!` macro is a declarative language to help you to build and declare your reactive UI quickly and expressively. It allows user interfaces to be described in terms of their visual structure, and almost all its syntaxes are expanded to the syntax of Rusts. So don't worry about the learning curve of this DSL language.

`widget!` is a macro that returns a widget, so you can use it as an expression anywhere you want.

Let's learn its syntax by building a greeting application.

## Nested struct literal syntax to describe widget hierarchy

The `widget!` macro describes a widget by using the Rust struct literal syntax.

```rust ignore
use ribir::prelude::*;

fn main() {
  let hi = widget! {
    Text { text: "Hello world!" }
  };

  app::run(hi);
}
```

At first, we import `ribir::prelude::*`, which is the list of essential things that a `Ribir` application needs to use. In the `main` function, we declare a `hi` widget and use it as a root widget to run an application. If you run `cargo run`, a window with text `Hello world!` will launch.

`Text` is a widget provided by Ribir, which displays simple text. It has two fields: the `text` used to specify what text to display, and the `style` specify the style of the text. Here we only use `text` and `style` will be a default value.

> Tips
>
> Any struct can support declare in `widget!` if it is derived `Declare` trait. `Declare` provide the default value for the `style` of `Text`. See more in [How `Declare` trait work [wip] ?](./).

In the next step we want it to say hi to anyone, by entering the name to say hi to. So we need an `Input` widget to accept user input.

```rust 
  let hi = widget! {
    Column {
      Input { 
        PlaceHolder::new("Enter the name you want to greet.")
      }
      Text { text: "Hello world!" }
    }
  };
```

With `widget!` user interfaces are described in terms of their visual hierarchy. The above code declares a `Column` widget that has two children `Input` and `Text` and a `PlaceHolder` that is the child of the `Input` widget. The child is always declared nested in the parent widget. With the exception of struct literal syntax, we allow describing leaf widgets across tuple struct or function call, like `PlaceHolder` in here.

For now, we have an `Input` to accept user input and use a `Column` to place the `Input` and `Text` widgets vertically. But the rendered text of the `Text` widget still does not update when the user input changes.

## Use id to access and directly react to the widget modifier.

Every object declared in `widget!` can be named using the `id` field. The `id` must be unique in the whole `widget!` scope. After a widget is named with an `id`, its `id` can be directly accessed like a smart pointer in `widget!` or embed `widget!`. 

`id` implicitly describes the relationship between objects. For example, if a field of an object initializes with an expression referencing `id`, it means the field value will react to changes of `id` to update.

```rust 
let hi = widget! {
  Column {
    Input {
      id: input,
      Placeholder::new("Enter the name you want to greet.")
    }
    Text { 
      text: {
        let value = &*input.text();
        format!("Hello {value}!")
      } 
    }
  }
};
```

We named the `Input` with `input`, and used its text to format content for `Text`. Now, the `Text` widget will react to user input.

> **Tips**
>
> We did not directly format the text in one line like `format!("Hello {}!", input.text())`, because `Ribir will not go deep into the external macro to analyze. So if we write `input.text()` in a macro, the expression result will not react to the changes of `input`.

## Built-in abilities compose to any widget.

After this works as we expected, let's try to beautify our style.

```rust 
let hi = widget! {
  Column {
    Input {
      id: input,
      h_align: HAlign::Center,
      Placeholder::new("Enter the name you want to greet.")
    }
    Text { 
      text: {
        let value = &*input.text();
        format!("Hello {value}!")
      },
      h_align: HAlign::Center,
      v_align: VAlign::Center,
    }
  }
};
```

Now, we center `Input` horizontally and center the `Text` both horizontally and vertically by using `h_align` and `v_algin` properties.

But wait, where do `h_align` and `v_align` come from? Both `Input` and `Text` did not declare these fields. That is because `Input` and `Text` are widgets provided by Ribir. There are dozens more of these built-in widgets. The fields and methods of the built-in widgets can directly be used like fields declared manually using the `widget!` macro. See the [full list of built-in fields][builtin] to for an overview what is available.

> **Tips**
>
>Ribir provides some built-in fields to extend the struct literal syntax. But remember **widget fields and built-in fields do not belong to the same widget**. In essence, they work together in a composed way, which means if the user uses a built-in field in a widget, the built-in widget composes the widget into a new widget. In this way, any widget gets many abilities from built-in widgets and pays the memory overhead only if the user use it. 

## Use states to declare more stateful objects.

This app immediately updates the greet text after the user enters a char, even if the name is not entered completely yet. In this section, we'll add a button to let the user explicitely submit the name after completing it and also count how many people we have already greeted.

```rust 
let hi = widget! {
  states { counter: Stateful::new(0) }
  Column {
    Row {
      align_items: Align::Center,
      Input {
        id: input,
        Placeholder::new("Enter the name you want to greet.")
      }
      Button {
        tap: move |_| {
          let name = &*input.text();
          greet.text = format!("Hello {name}!").into();
          input.set_text("");
          *counter += 1;
        },
        ButtonText::new({
          let counter = counter.to_string();
          format!("Greet!({counter})")
        })
      }
    }
    Text { 
      id: greet,
      text: "Hello world!",
      h_align: HAlign::Center,
      v_align: VAlign::Center,
    }
  }
};
```

The most important change is that we add `states { counter: Stateful::new(0) }` at the top. We call it `states` block. It starts with a keyword `states` and declares a stateful object in a `key: value` mode. When the stateful object is declared in this block modifier, it'll also be reacted to by others that refer to it in the `widget` macro, and behaves like with an object referred to by `id`.

The other change is that we add the `Button` after the `Input`, and update the `Input`, `Text` and `counter` when the user taps it.

The `states` block it is very useful to tell the `widget!` macro that we have other stateful objects that need to be reactive. Here, we defined a stateful object `counter` and used it as a part of the `Button` text. We declare the counter as stateful, so that when the `counter` changes, after the user tapped the button, the button text will auto-update.

## Use `DynWidget` to declare a dynamic widget hierarchy.

You may have found that our application has a static widget tree, and even if the user not enter anything, a "Hello world!" was displayed. In this section, we'll introduce a special widget `DynWidget` and use it to generate the `greet` widget conditionally.

`DynWidget` has a `dyns` field to accept dynamic widgets. The `dyns` value will replace `DynWidget` itself as its parent child or children. When `dyns` is modified, the widgets are updated. The `dyns` value type is limited by `DynWidget`'s parent. Usually it can be one of three kinds:

- The child type the `DynWidget`'s parent requires. 
- An `Option` of `C`, if `DynWidget`'s parent accepts one or multiple `C` as its child.
- A type that implements `IntoIterator` and its iterate item is `C`, if `DynWidget`'s parent accepts  multiple `C` as its children.

Now, we use `DynWidget` to control if we need a greet `Text`:

```rust
DynWidget {
  dyns: (*counter > 0).then(|| {
    widget! {
      Text { 
        id: greet,
        text: "Hello world!",
        h_align: HAlign::Center,
        v_align: VAlign::Center,
      }
    }
  })
}
``` 

This `DynWidget` generates an optional widget when it detects that the `counter` is greater than zero. One thing that could be improved is to not always regenerate `greet` after the `counter` changes. Indeed the `greet` widget should regenerate only after the result `*counter > 0` changed the first time. Let's go further:

```rust
DynWidget {
  dyns := assign_watch!(*counter > 0)               // edit!
    .stream_map(|o| o.distinct_until_changed())     // new!
    .map(move |need_greet| {                        // new!
      need_greet.then(|| {
          widget! {
            Text { 
              id: greet,
              text: "Hello world!",
              h_align: HAlign::Center,
              v_align: VAlign::Center,
            }
          }
        })
    })
}
```

We added three lines of code. 

The first line is `dyns := assign_watch!(*counter > 0)`, we use operator `:=` instead of `:` to initialize the `dyns`. Unlike `:`, `:=` accepts an `AssignObservable` as its initialization value and explicitly subscribes to it to update the field. `AssignObservable` is a type that contain the initialization value and an observable stream of that value. The `assign_watch!` macro is used to convert a expression to an `AssignObservable`. 

In the second line we use `stream_map` to chain `distinct_until_changed` on the stream observable. So we accept the changes only when the result of `*counter > 0` changed the first time.

The third line `.map(move |need_greet| {...}) ` maps a `bool` to `Option<Widget>` what `dyns` want.


> **Tips**
>
> `DynWidget` also supports accepting children depending on the type of `dyns` allow. Its children are static. The dynamic part only limits in `dyns` field.
>```rust
>Column {
>  DynWidget {
>    dyns: (*counter > 0)
>      // this `Row` is dynamic.
>      .then(||Row::default())
>    // this `Row` is always exist.
>    // if the 'dynamic' Row is Some-value, this `Row` is the child of it. 
>    // otherwise, it's the child of the `Column`
>    Row {}
>  }
>}
>```


Let's put all code together:

```rust
use ribir::prelude::*;

fn main() {
  let hi = widget! {
    states { counter: Stateful::new(0) }
    Column {
      Row {
        align_items: Align::Center,
        Input {
          id: input,
          Placeholder::new("Enter the name you want to greet.")
        }
        Button {
          tap: move |_| {
            let name = &*input.text();
            greet.text = format!("Hello {name}!").into();
            input.set_text("");
            *counter += 1;
          },
          ButtonText::new({
            let counter = counter.to_string();
            format!("Greet!({counter})")
          })
        }
      }

      DynWidget {
        dyns: (*counter > 0).then(|| {
          widget! {
            Text {
              id: greet,
              text: "Hello world!",
              h_align: HAlign::Center,
              v_align: VAlign::Center,
            }
          }
        })
      }
    }
  };

  app::run(hi);
}
```

When we compile, the compiler complaines about not finding `greet`:

```
error[E0425]: cannot find value `greet` in this scope
  --> ribir/examples/greet.rs:16:13
   |
16 |  greet.text = format!("Hello {name}!").into();
   |  ^^^^^ not found in this scope
```

This is because the `tap` callback of `Button` is trying to access `greet`, but `greet` is declared in an embedded `widget!`, in `DynWidget`. `widget!` can access any `id` in the same `widget!` scope or outside `widget!`, but not in an embedded `widget!`. In this case, when we think deeply, we'll find that `greet` does not always exists for the outside `widget!` scope.

In the next section, we'll resolve it across that access `Button` and `input` in the embed `widget!` instead of access `greet` text in the outside `widget!`.

## `init` and `finally` block

As before, we introduced `states` block to declare more stateful objects. There are two other blocks we can use in `widget!`, that `init` and `finally` block.

`init` and `finally` are blocks only accept statements and run these statements in `widget!`.

`init` run statements after the `states` block and before any others in `widget!`, and the variable in `init` block can be accessed in the whole `widget!` scope except for `states`.

`finally` run statements after everything declare in `widget!` scope but before `widget!` compose the final widget to return.

### Subscribe `tap` event in `finally` block

Back to our greet app, first, we update the `tap` callback and only use it to update the `counter` and give a name `submit` to `Button`, so we can access it in the embed `widget!`.

```rust
Button {
  id: submit,
  tap: move |_| *counter += 1,
  ButtonText::new({
    let counter = counter.to_string();
    format!("Greet!({counter})")
  })
}
```

Then we add `finally` block in the embed `widget!`, and subscribe to the tap stream of `submit` to update the `greet` text and reset the `input`.

```rust
finally {
  let guard = submit
    .tap_stream()
    .subscribe(move |_| {
      let name = &*input.text();
      greet.text = format!("Hello {name}!").into();
      input.set_text("");
    })
    .unsubscribe_when_dropped();
  move_to_widget!(guard);
}
```

Let’s review this `finally` block in detail.

`finally` is the keyword of the block, all statements wrapped by `{}` it as part of the block.

In this block, there are two statements. Let deep in one by one.

In the first statement, `submit.tap_stream()` return the tap event as a ReactiveX `Observable` stream, then we subscribe it to update `greet` text and reset `input`. The name of `unsubscribe_when_dropped` is self-described, it converts the handle to a variable that will make auto `unsubscribe` when it dropped. So the whole statement subscribes to the tap event of `submit` and create a variable to manage the subscribe lifetime.

The next statement `move_to_widget!(guard);` is very simple. `move_to_widget!` move `guard` to `widget!`, in other words, transfer the ownership of the `guard`. So the `guard` will live as long as `widget!`, at here it's the `greet`.

> **Tips**
>
> - ReactiveX is an API for asynchronous programming with observable streams. [See More](https://reactivex.io)
> - rxRust is the implementation of ReactiveX Ribir used. [See More](https://github.com/rxRust/rxRust)

### use `watch` and `let_watch!` to watch expression.

In `finally` block, we subscribe to the `tap` event stream. Here we provide another way to do the same thing. And we will introduce `watch!` and `let_watch!` macro. They're very useful macros that let we can subscribe to the modifications of an expression result.

We already have a stateful object `counter`, and it increments itself after every tap of `submit` button. So we can update the `greet` text after the `counter` modifies. We'll use `watch!` macro to do it. `watch!` helps us to convert an expression to a value stream.

> Tips
>
> `watch!` macro subscribe all stateful objects in the expression and recalculates it when any stateful object modifies. 

We use `watch!(counter)` instead of `submit.tap_stream()` now.

```rust
finally {
  let guard = watch!(counter)
    .subscribe(move |_| {
      let name = &*input.text();
      greet.text = format!("Hello {name}!").into();
      input.set_text("");
    })
    .unsubscribe_when_dropped();
  move_to_widget!(guard);
}
```

`let_watch!` is a more convenient macro if you want to subscribe an expression. Like `watch!`, it converts an expression to a value stream, but also auto unsubscribe this when the root of `widget!` over. We use `let_watch!` instead of `watch!`, `.unsubscribe_when_dropped()` and `move_to_widget!(guard)` not necessary anymore.

```rust
finally {
  let_watch!(counter)
    .subscribe(move |_| {
      let name = &*input.text();
      greet.text = format!("Hello {name}!").into();
      input.set_text("");
    });
}
```

> Tips
>
> In most cases, `let_watch!` is a more convenient, but `watch!` is more flexible in some complex situations.

### Access `BuildCtx` in `init` block

Our `greet` text with a default style. In this section, we'll use larger letters to highlight it. Instead of hard code for greet `style`, we'll use the `Theme` to init it. 

First, let's add an `init` block after `states`

```rust
init ctx => {
  let style = TypographyTheme::of(ctx).headline1.text.clone();
}

```
`init` block has the same syntax as `finally`, but the above code has a little more stuff (`ctx =>`) than what the `finally` block we used before.

`ctx =>` is a syntax we use to name `BuildCtx`. Here, `ctx` is the name of `BuildCtx`. Because the `finally` block not use `BuildCtx`, so it be omitted. Let deep into the body, we create a style variable, and init it by the predefined theme.

Second, we use `style` to init the `greet` text.

``` rust
Text { 
  id: greet,
  text: "Hello world!",
  style,  // new line.
  h_align: HAlign::Center,
  v_align: VAlign::Center,
}
```

> **Tips**
>
> - `TypographyTheme` configures the text style used in the application as a part of `Theme`. Click [`Introduction to Theme[wip]`] to learn more.
> - `BuildCtx` provides many contexts for the application of the `widget!`.

## Animations

We build a greet application, but the greet text transition without animation looks blunt. In this section, we will add animations to transition the greet text and cover the basic knowledge of Ribir animations.

At first, we need to split `greet` as three `Text`, because the `Hello` and `!` part will never change. Our animation only works on the name part.

We change the single `Text`

``` rust
Text { 
  id: greet,
  text: "Hello world!",
  style,
  h_align: HAlign::Center,
  v_align: VAlign::Center,
}
```

To a `Row` widget with three `Text`, and apply `h_align` and `v_align` to `Row`.

``` rust
Row {
  h_align: HAlign::Center,
  v_align: VAlign::Center,
  Text { text: "Hello ", style: style.clone() }
  Text {
    id: greet,
    text: "World",
    style: style.clone()
  }
  Text { text: "!" , style }
}
```

Then, we can directly update the `greet` text by the text of `input`.

```rust
finally {
  let_watch!(counter)
    .subscribe(move |_| {
      greet.text = input.text();
      input.set_text("");
    });
}
```

After the widgets are ready, we will add a "ease in" animation to transition `greet` text change. Ribir does animate all across the `Animate` object. Let's declare an `Animate` object after the `Row` widget.

```rust
Animate {
  id: greet_new,
  transition: Translation {
    delay: None,
    duration: Duration::from_mills(200),
    easing: easing::EASE_IN,
    repeat: None,
  },
  prop: prop!(greet.transform),
  from: Transform::translation(0., greet.layout_height() * 2.)
}
```

We have already introduced `id` before. Here, we use it to name the object `greet_new`. `Animate` has three public fields `transition`, `prop` and `from`. These three fields described how an animation runs. 

- `transition` field accepts a type that implemented `Roc` (Rate of change) trait, Ribir provides `Transition` as the standard implementation. We use `Transition` to describe how property animates smoothly from another value to the current value over time. 
> **Tips**
>
> `Transition` also supports declaring individual, `Animate` can use it across its `id`.
>```rust
>Transition {
>   id: ease_in,
>   duration: Duration::from_mills(200),
>   easing: easing::EASE_IN,  
>}
>Animate {
>  id: greet_new,
>  transition: ease_in.clone_stateful(),
>  prop: prop!(greet.transform),
>  from: Transform::translation(0., greet.layout_height() * 2.)
>}
>```

- `prop` field accepts the animate property need transition, the animate property is a type that implemented `AnimateProperty`. Here we want to do animate for `transform` of `greet`. `prop!(greet.transform)` help us to construct an animate property by a chaining path of field, and the path must start with a stateful object.

> **Tips**
>
> - `transform` is a built-in field, so we can directly use it even if it's not a field of `Text`.
> - In fact, `prop!` also has a second argument called lerp function. Lerp function requires to implement `Fn(&P, &P, f32) -> P`, it used to interpolate between two property value linearly. If we provide a lerp function argument, that means this property should use this function to calculate interpolate value. Otherwise, the type of property must implement `Lerp`. So we can write the property as
> ```rust
>// Do same thing as `prop!(greet.transform)`.
>prop!(greet.transform, |from, to, rate| from.lerp(to, rate))
>```
> - We also can use tuple to group multi property. For example 
>```rust ignore
>Animate {
>   transition: ..,
>   prop: (
>     prop!(greet.transform), 
>     prop!(greet.opacity),
>   ), 
>   from: (Transition::scale(0., 0.), 0.)
>}
>
>```

- `from` field accept a value of a property which the animate start from.

Someone may need clarification, we have a `from` field describe animate come from, but why not have a `to` field describe where to go. In Ribir, animations are only visual effects and should not affect any application data. Ribir animate always finished at the value of the animate property, that's where the animation goes.

> Tips
>
> Because animations are only visual effects, the animate property must be a field of `Render` widget.

We have learned how to declare an `Animate` object. The next step is to trigger it. We need to trigger it when the `greet` text is modified, so add this statement to `finally` block.
```rust
let_watch!(greet.text)
  .subscribe(move |_| greet_enter.run());
```

The animation work as we expected now. In practice, we use the predefined `Transition` in theme. We polish over animate code.

Add a `init` block, before `Row` widget.

```
init ctx => {
  let ease_in = transitions::EASE_IN.of(ctx);
}
```
Then use `ease_in` to initialize the `Animate`.

```
Animate {
  id: greet_new,
  transition: ease_in,
  prop: prop!(greet.transform),
  from: Transform::translation(0., greet.layout_height() * 2.)
}
```

Let's review all the code of the widget.

```rust 
 let hi = widget! {
    states { counter: Stateful::new(0) }
    init ctx => {
      let style = TypographyTheme::of(ctx).headline1.text.clone();
    }
    Column {
      Row {
        align_items: Align::Center,
        Input {
          id: input,
          Placeholder::new("Enter the name you want to greet.")
        }
        Button {
          tap: move |_| *counter += 1,
          ButtonText::new({
            let counter = counter.to_string();
            format!("Greet!({counter})")
          })
        }
      }
      DynWidget {
        dyns: := assign_watch!(*counter > 0)
          .stream_map(|o| o.distinct_until_changed())
          .map(move |need_greet| {
            need_greet.then(|| {
              widget! {
                init ctx => {
                  let ease_in = transitions::EASE_IN.of(ctx);
                }
                Row {
                  Text { text: "Hello ", style: style.clone() }
                  Text {
                    id: greet,
                    text: "World",
                    style: style.clone()
                  }
                  Text { text: "!" , style }
                }
                Animate {
                  id: greet_new,
                  transition: ease_in,
                  prop: prop!(greet.transform),
                  from: Transform::translation(0., greet.layout_height() * 2.)
                }
                finally {
                  let_watch!(counter)
                    .subscribe(move |_| {
                      greet.text = input.text();
                      input.set_text("");
                    });
                  let_watch!(greet.text)
                    .subscribe(move |_| greet_new.run());
                }
              }
            })
          })
      }
    }
  };
```

> **Tips** 
> 
>In addition to a "standard" trigger animation that declares an `Animate` and then manually triggers it, there is a syntax sugar to quick transition property change. It starts with a keyword  `transition`. It gives an animate effect when its value changes.
>
>```rust ignore
>transition prop!(greet.background) {
>  duration: Duration::from_mills(200),
>  easing: easing::EASE_IN,
>}
>```
>
>The animate property follows the `transition` keyword, and the next block declares a `Transition` object.
>
>Instead of declaring a new `Transition` object, we also can use `by` field to provide a `Transition` object by an expression.
>
>```rust ignore
>transition prop!(greet.background) {
>  // assume `ease_in` is a `Transition` variable.
>  by: ease_in
>}
>```

That's all, we've covered all the syntax of `widget!`. You can find the code in [github](https://github.com/RibirX/Ribir/blob/master/ribir/examples/greet.rs). And this's just a `widget!` syntax teaching demo, no consideration to its completeness and reasonableness. In practices, use a `visible` to control `greet` show or hide is a easier and better way.

[builtin]: ../builtin_widget/declare_builtin_fields.md
