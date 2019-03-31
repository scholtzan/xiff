//! A plugin to show git diff in xi-editor.
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;

use std::path::Path;

use crate::xi_core::ConfigTable;
use xi_plugin_lib::{mainloop, ChunkCache, Plugin, View};
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;
use crate::xi_core::rpc::CoreNotification;

struct XiffPlugin {
}

impl Plugin for XiffPlugin {
    type Cache = ChunkCache;

    fn new_view(&mut self, view: &mut View<Self::Cache>) {

    }

    fn did_close(&mut self, _view: &View<Self::Cache>) {}

    fn did_save(&mut self, _view: &mut View<Self::Cache>, _old: Option<&Path>) {}

    fn config_changed(&mut self, _view: &mut View<Self::Cache>, _changes: &ConfigTable) {}

    fn update(
        &mut self,
        view: &mut View<Self::Cache>,
        _delta: Option<&RopeDelta>,
        _edit_type: String,
        _author: String,
    ) {

    }
}

impl XiffPlugin {
    fn new() -> XiffPlugin {
        XiffPlugin {

        }
    }
}

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
