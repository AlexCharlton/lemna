use lemna_nih_plug::nih_plug::prelude::*;

use plugin_example_hello::M8Plug;

fn main() {
    nih_export_standalone::<M8Plug>();
}
