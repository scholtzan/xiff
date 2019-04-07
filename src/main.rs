//! A plugin to show git diff in xi-editor.
extern crate git2;
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;

use git2::{DiffOptions, Repository};
use std::path::Path;

use crate::xi_core::annotations::AnnotationType;
use crate::xi_core::plugin_rpc::DataSpan;
use crate::xi_core::ConfigTable;
use git2::DiffLine;
use serde_json::json;
use xi_plugin_lib::{mainloop, ChunkCache, Plugin, View};
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;
use xi_rope::delta::DeltaRegion;

struct XiffPlugin {
    unsaved_inserts: Vec<usize>,
    unsaved_deletions: Vec<usize>,
}

impl Plugin for XiffPlugin {
    type Cache = ChunkCache;

    fn new_view(&mut self, view: &mut View<Self::Cache>) {
        self.update_diff(view, Interval::new(0, view.get_buf_size()));
    }

    fn did_close(&mut self, _view: &View<Self::Cache>) {}

    fn did_save(&mut self, _view: &mut View<Self::Cache>, _old: Option<&Path>) {
        self.unsaved_inserts = Vec::new();
        self.unsaved_deletions = Vec::new();
    }

    fn config_changed(&mut self, _view: &mut View<Self::Cache>, _changes: &ConfigTable) {}

    fn update(
        &mut self,
        view: &mut View<Self::Cache>,
        delta: Option<&RopeDelta>,
        _edit_type: String,
        _author: String,
    ) {
        if let Some(delta) = delta {
            for DeltaRegion { old_offset, .. } in delta.iter_deletions() {
                let line = view.line_of_offset(old_offset).unwrap();

                if !self.unsaved_deletions.contains(&line) {
                    self.unsaved_deletions.push(line);
                }
            }

            for DeltaRegion { new_offset, .. } in delta.iter_inserts() {
                let line = view.line_of_offset(new_offset).unwrap();

                if !self.unsaved_deletions.contains(&line) {
                    self.unsaved_inserts.push(line);
                }
            }
        }

        self.update_diff(view, Interval::new(0, view.get_buf_size()));
    }
}

impl XiffPlugin {
    fn new() -> XiffPlugin {
        XiffPlugin {
            unsaved_inserts: Vec::new(),
            unsaved_deletions: Vec::new(),
        }
    }

    fn lines_to_data_span(&self, lines: Vec<usize>, view: &mut View<ChunkCache>) -> Vec<DataSpan> {
        let mut i = 0;
        let mut spans: Vec<DataSpan> = Vec::new();
        let mut sorted_lines = lines.clone();
        sorted_lines.sort();

        while i < sorted_lines.len() {
            let start = sorted_lines.get(i).unwrap();

            while i < sorted_lines.len() - 1 && sorted_lines.get(i + 1).unwrap() - 1 <= *sorted_lines.get(i).unwrap() {
                i = i + 1;
            }

            spans.push(DataSpan {
                start: view.offset_of_line(*start).unwrap(),
                end: view.offset_of_line(*sorted_lines.get(i).unwrap() + 1).unwrap() - 1,
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
                ).expect("Error parsing diff");

                inserted_lines.extend(&self.unsaved_inserts);
                deleted_lines.extend(&self.unsaved_deletions);

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
