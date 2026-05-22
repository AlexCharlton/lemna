// Uncomment to remove the terminal window associated with the application on Windows
// Will also disable printing to a terminal.
// #![windows_subsystem = "windows"]
use nice_plug::prelude::*;

use plugin_example_hello::HelloPlugin;

fn main() {
    nice_export_standalone::<HelloPlugin>();
}
