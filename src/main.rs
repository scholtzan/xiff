//! A plugin to show git diff in xi-editor.
extern crate git2;
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;

use git2::{Commit, DiffOptions, ObjectType, Repository, Signature, Time};
use git2::{DiffFormat, Error, Pathspec};
use std::cmp::max;
use std::path::Path;
use std::str;

use crate::xi_core::annotations::AnnotationType;
use crate::xi_core::plugin_rpc::DataSpan;
use crate::xi_core::rpc::CoreNotification;
use crate::xi_core::ConfigTable;
use git2::{DiffHunk, DiffLine};
use serde_json::json;
use xi_plugin_lib::{mainloop, ChunkCache, Plugin, View};
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;

struct XiffPlugin {}

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
        XiffPlugin {}
    }

    fn lines_to_data_span(&self, lines: Vec<usize>, view: &mut View<ChunkCache>) -> Vec<DataSpan> {
        let mut i = 0;
        let mut spans: Vec<DataSpan> = Vec::new();

        while i < lines.len() {
            let start = lines.get(i).unwrap();

            while i < lines.len() - 1 && lines.get(i + 1).unwrap() - 1 == *lines.get(i).unwrap() {
                i = i + 1;
            }

            spans.push(DataSpan {
                start: view.offset_of_line(*start).unwrap(),
                end: view.offset_of_line(*lines.get(i).unwrap() + 1).unwrap() - 1,
                data: json!(null),
            });

            i = i + 1;
        }

        spans
    }

    fn update_diff(&mut self, view: &mut View<ChunkCache>, interval: Interval) {
        if let Some(path) = view.get_path() {
            if let Ok(repo) = Repository::open(path.parent().unwrap()) {
                let mut opts = DiffOptions::new();
                opts.new_prefix(path)
                    .include_ignored(false)
                    .include_untracked(false);

                let diff = repo.diff_index_to_workdir(None, Some(&mut opts)).unwrap();

                let mut inserted_lines: Vec<usize> = Vec::new();
                let mut deleted_lines: Vec<usize> = Vec::new();

                diff.foreach(
                    &mut |_, _| true,
                    None,
                    None,
                    Some(&mut |_delta, _hunk, line: DiffLine| {
                        match line.origin() {
                            '+' => {
                                inserted_lines.push(line.new_lineno().unwrap() as usize);
                            }
                            '-' => {
                                deleted_lines.push(line.old_lineno().unwrap() as usize);
                            }
                            _ => {}
                        };

                        true
                    }),
                );

                let (modified_lines, inserted_lines): (Vec<usize>, Vec<usize>) = inserted_lines
                    .into_iter()
                    .partition(|l| deleted_lines.contains(l));

                let (_, deleted_lines) = deleted_lines
                    .into_iter()
                    .partition(|l| modified_lines.contains(l));

                let inserted_spans = self.lines_to_data_span(inserted_lines, view);
                let deleted_spans = self.lines_to_data_span(deleted_lines, view);
                let modified_spans = self.lines_to_data_span(modified_lines, view);

                view.update_annotations(
                    interval.start(),
                    interval.end(),
                    &inserted_spans,
                    &AnnotationType::Other("added".to_string()),
                );
                view.update_annotations(
                    interval.start(),
                    interval.end(),
                    &deleted_spans,
                    &AnnotationType::Other("deleted".to_string()),
                );
                view.update_annotations(
                    interval.start(),
                    interval.end(),
                    &modified_spans,
                    &AnnotationType::Other("modified".to_string()),
                );
            }
        }
    }
}

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
