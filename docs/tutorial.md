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
This example defines a struct called `App` and makes it a [`Component`], which is the central Lemna interface. There's no special meaning to the name "App", it's just a convention for the top-level or "root" Component in your application. And the only special thing about a root Component is that it must implement [`std::default::Default`].

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

You'll notice that the example lives in the `lemna-baseview` project. This's because Lemna is designed to be able to run in any "backend" that implements the [`Window`] trait. Backends handle opening a window, receiving events like mouse clicks, and other interactions with your OS's window manager. `lemna-baseview` uses a (forked) [baseview](https://github.com/AlexCharlton/baseview) backend, and it's by far the most functional backend today.

Since all of our examples use more or less the same `main` function, we will elide it in the future unless there's a reason not to. **Any time we define a `main` function it's then used in all the succeeding examples until it's again redefined.** You can find all tutorial examples [here](https://github.com/AlexCharlton/lemna/tree/main/backends/baseview/examples/tutorial).

## An introduction to Components and Nodes
Individually, a [`Component`] can do a couple of neat things: They can draw to the screen, and handle any events that gets passed their way. But it's when you combine them with other Components that things really start to cook.

In Lemna, you never create a `Component` on its own, you always attach it to a [`Node`]. `Node`s hold an instance of a `Component` as well as a [`Layout`][layout::Layout], which tells Lemna where to position this `Component` instance. The [`view`][Component#view] method of a `Component` is used to define what child `Node` it will create. You can also [`push`][Node#push] a child `Node` onto another (as long as it's a Node holding a ["container"][Component#container] Component). You can [`push`][Node#push] as many child nodes onto a `Node` as you like. In doing so, you construct the graph that represents your application.

Let's see what this looks like with an example. Here we have an App that isn't too different from the first, but we're defining a new `Component` called `BlueBorder`. A `BlueBorder` `Node` will wrap any of its child `Nodes` in a blue `Div`. The `Div` has a `padding` of 10 pixels specified, so our "border" will be 10 pixels wide.

```
use lemna::{widgets::*, *};

#[derive(Debug)]
pub struct BlueBorder {}
impl Component for BlueBorder {
    // `BlueBorder` creates a blue `Div` child Node
    fn view(&self) -> Option<Node> {
        Some(node!(
            Div::new().bg(Color::BLUE),
            [padding: [10]]
        ))
    }

    // This Component is a "container". Children get pushed onto the index specified.
    // In this case, any Nodes pushed onto a `BlueBorder` will get added to the
    // child at index `0`. That's the `Div` that is returned by `view`.
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
              .push(node!(BlueBorder {})
                      .push(node!(Div::new().bg(Color::RED), [size: [100]])))
              .push(node!(BlueBorder {})
                      .push(node!(Div::new().bg(Color::GREEN), [size: [100]]))),
        )
    }
}

```

When we run it, it looks like this:

![Example 2][tut2]
<br />_Example: tut2_

The graph of Nodes that gets created by this application looks like this:

![The Nodes created by the example][nodes]
<br />_The graph of Nodes that are constructed by the previous example. We've colored Nodes that were instantiated by the `App` in green, while Nodes instantiated by `BlueBorder` are blue._

When Lemna needs to update a running app, it calls [`view`][Component#view] on the root `Node` (`App`). Lemna then calls `view` on all of the children of the `Node`s returned by the root, and then recursively continues calling `view` on _their_ children. In other words, it creates a fresh graph with every update. For this reason, you should never do anything too computationally expensive in the `view` method. We'll talk about where you can do that sort of computation later.

You can probably tell that this makes `view` a very important method for `Component`s. Almost all of the Components you define will output Nodes through that `view` method. The only Components that don't are either pure "containers" -- `Component`s designed specifically to hold child `Node`s that get `push`ed onto them, like `BlueBorder` as well as `Div` -- or they're leaf Nodes that draw to the window using the [`render`][Component#render] method, which we'll also discuss later.

⚠️ _We'll often talk about `Component`s and `Node`s interchangeably, because of their 1:1 relationship with one another. For instance, we said above that we call [`view`][Component#view] on a `Node`, but we really meant, "we call [`view`][Component#view] on the `Component` instance that belongs to the `Node`."_


We'll end this section by illustrating how we refer to the relationships between Nodes in our application. If we consider a single `BlueBorder` Node, we name its relationships thusly:

![The relationships between Nodes created by the example][relationships]


We'll use this language throughout the documentation.

## Layouts -- Positioning Components relative to each other

## Intro to handling Events
TODO: Create a component that handles events and passes messages back to the app

## Creating stateful Components
TODO: Create an app where you click on a button and it randomizes how many boxes are displayed. This will also illustrate [`key`][Node#key].

## Widgets: Built-in Components

## Styling

## Advanced Event handling -- Interacting with the Window

## Outputting rendering primitives with Renderables
