use lemna_nih_plug::nih_plug::prelude::*;

use plugin_example_hello::HelloPlugin;

fn main() {
    nih_export_standalone::<HelloPlugin>();
}
