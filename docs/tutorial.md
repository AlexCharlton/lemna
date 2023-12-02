# Guided tour

## Running an app in a Window
Lemna borrows a lot of ideas from [React](https://react.dev/), so if you've used that before, Lemna should feel familiar. You won't see any HTML or CSS, however, and compiling a Lemna app results in a stand-alone binary that doesn't need to have a whole web rendering and JavaScript stack compiled into it.

Let's look at a very basic Lemna application, so we can get a taste:

```
use lemna::{widgets::*, *};

#[derive(Debug, Default)]
pub struct App {}

impl Component for App {
    fn view(&self) -> Option<Node> {
        Some(node!(
            Div::new().bg(0xFF00FFFF),
            [size: [100, 100]]
        ))
    }
}

fn main() {
    lemna_baseview::Window::open_blocking::<App>(lemna_baseview::WindowOptions::new(
        "Hello",
        (400, 300),
    ));
}
```

This example defines a struct called `App` and makes it a [`Component`], which is the central Lemna interface. There's no special meaning to the name "App", it's just a convention for the top level Component in your application.

We define the `view` method of `App` to return a [`Node`], which is constructed with the [`node!`][macro@node] macro. Our Node contains another `Component` named [`Div`][widgets::Div], which is named after the [HTML element](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div). We set the `Div`'s background to the hex color `0xFF00FFFF` (a.k.a. magenta), and give it a width and height of 100 pixels.

Then, in our `main` function, we open a window that contains our `App`. The result looks like this:

![Example 1][tut1]
<br />_Example: tut1_

The rest of the tutorial will explain in a lot more detail what the deal is with Components and Nodes, and how you use them to create fully interactive applications.

### Running tutorial examples
You can run any example in this tutorial like this:
```shell
$ cargo run -p lemna-baseview --example tut1
```
Replacing the name of the example with the name provided.

Since all of our examples use the same `main` function, we will elide it in the future, but you can find all tutorial examples [here](https://github.com/AlexCharlton/lemna/tree/main/backends/baseview/examples/tutorial).

## An introduction to Components

### Nodes
Nodes are combined via calls to a Component's [`#view`][Component#view] method to form the graph that is a given application.

### Layout

## Intro to handling Events

## Creating stateful Components

## Widgets: Built-in Components

## Styling

## Advanced Event handling -- Interacting with the Window

## Outputting rendering primitives with Renderables
