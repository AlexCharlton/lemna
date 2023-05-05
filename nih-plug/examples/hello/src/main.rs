// Uncomment to remove the terminal window associated with the application on Windows
// Will also disable printing to a terminal.
// #![windows_subsystem = "windows"]
use lemna_nih_plug::nih_plug::prelude::*;

use plugin_example_hello::HelloPlugin;

fn main() {
    nih_export_standalone::<HelloPlugin>();
}
