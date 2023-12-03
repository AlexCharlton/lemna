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
_Example: tut1_

This example defines a struct called `App` and makes it a [`Component`], which is the central Lemna interface. There's no special meaning to the name "App", it's just a convention for the top level Component in your application.

We define the `view` method of `App` to return a [`Node`], which is constructed with the [`node!`][macro@node] macro. Our Node contains another `Component` named [`Div`][widgets::Div], which is named after the [HTML element](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div). We set the `Div`'s background to the hex color `0xFF00FFFF` (a.k.a. magenta), and give it a width and height of 100 pixels.

Then, in our `main` function, we open a window that contains our `App`. The result looks like this:

![Example 1][tut1]

The rest of the tutorial will explain in a lot more detail what the deal is with Components and Nodes, and how you use them to create fully interactive applications.

### Running tutorial examples
You can run any example in this tutorial like this:
```shell
$ cargo run -p lemna-baseview --example tut1
```
Replacing the name of the example with the name provided.

You'll notice that the example lives in the `lemna-baseview` project. This's because Lemna is designed to be able to run in any "backend" that implements the [`Window`] trait. Backends handle opening a window, receiving events like mouse clicks, and other interactions with your OS's window manager. `lemna-baseview` uses a (forked) [baseview](https://github.com/AlexCharlton/baseview) backend, and it's by far the most functional backend today.

Since all of our examples use more or less the same `main` function, we will elide it in the future unless there's a reason not to. Any time we define a `main` function it's then used the all in the succeeding examples until it's again redefined. You can find all tutorial examples [here](https://github.com/AlexCharlton/lemna/tree/main/backends/baseview/examples/tutorial).

## An introduction to Components
TODO: overview of features

```
use lemna::{widgets::*, *};

#[derive(Debug)]
pub struct BlueBorder {}
impl Component for BlueBorder {
    fn view(&self) -> Option<Node> {
        Some(node!(
            Div::new().bg(Color::BLUE),
            [padding: [10]]
        ))
    }

    fn container(&self) -> Option<Vec<usize>> {
        Some(vec![0])
    }
}

#[derive(Debug, Default)]
pub struct App {}
impl Component for App {
    fn view(&self) -> Option<Node> {
        Some(
            node!(Div::new())
                .push(node!(BlueBorder {}).push(node!(Div::new().bg(Color::RED), [size: [100]])))
                .push(node!(BlueBorder {}).push(node!(Div::new().bg(Color::GREEN), [size: [100]]))),
        )
    }
}

```
_Example: tut2_

Nodes are combined via calls to a Component's [`#view`][Component#view] method to form the graph that is a given application.

## Layouts -- Positioning Components relative to each other

## Intro to handling Events
TODO: Create a component that handles events and passes messages back to the app

## Creating stateful Components
TODO: Create an app where you click on a button and it randomizes how many boxes are displayed

## Widgets: Built-in Components

## Styling

## Advanced Event handling -- Interacting with the Window

## Outputting rendering primitives with Renderables
