//! A plugin to show git diff in xi-editor.
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;
extern crate git2;

use std::path::Path;
use std::str;
use git2::{Repository, Signature, Commit, ObjectType, Time, DiffOptions};
use git2::{Pathspec, Error, DiffFormat};

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
        self.update_diff(view, Interval::new(0, view.get_buf_size()));
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
        self.update_diff(view, Interval::new(0, view.get_buf_size()));
    }
}

impl XiffPlugin {
    fn new() -> XiffPlugin {
        XiffPlugin {

        }
    }

    fn update_diff(&mut self, view: &mut View<ChunkCache>, interval: Interval) {
        if let Some(path) = view.get_path() {
            if let Ok(repo) = Repository::open(path.parent().unwrap()) {
                let mut opts = DiffOptions::new();
                opts.new_prefix(path)
                    .include_ignored(false)
                    .include_untracked(false);

                let diff = repo.diff_index_to_workdir(None, Some(&mut opts)).unwrap();

                // todo: translate into annotations
                diff.print(DiffFormat::Raw, |delta, _hunk, line| {
                    eprintln!("- {:?}", str::from_utf8(line.content()).unwrap());
                    true
                });
            }
        }
//        view.update_annotations(interval.start(), interval.end(),spans, annotation_type);
    }
}

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
