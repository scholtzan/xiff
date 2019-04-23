//! A plugin to show git diff in xi-editor.
extern crate diff;
extern crate git2;
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;

use git2::{DiffOptions, ObjectType, Repository};
use std::path::Path;
use std::cmp::max;

use crate::xi_core::annotations::AnnotationType;
use crate::xi_core::plugin_rpc::DataSpan;
use crate::xi_core::ConfigTable;
use serde_json;
use serde_json::json;
use std::str;
use xi_plugin_lib::{mainloop, ChunkCache, Plugin, View};
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;

struct XiffPlugin {}

#[derive(PartialEq, Debug)]
enum ChangeType {
    Insertion,
    Deletion,
    Modification,
}

impl Plugin for XiffPlugin {
    type Cache = ChunkCache;

    fn new_view(&mut self, view: &mut View<Self::Cache>) {
        view.schedule_idle();
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
        view.schedule_idle();
    }

    fn idle(&mut self, view: &mut View<Self::Cache>) {
        self.update_diff(view, Interval::new(0, view.get_buf_size()));
    }
}

impl XiffPlugin {
    fn new() -> XiffPlugin {
        XiffPlugin {}
    }

    fn get_head_content(&mut self, view: &mut View<ChunkCache>) -> Option<String> {
        if let Some(path) = view.get_path() {
            // get repository if current folder is part of one
            if let Ok(repo) = Repository::open(path.parent().unwrap()) {
                let mut opts = DiffOptions::new();
                opts.new_prefix(path)
                    .include_ignored(false)
                    .include_untracked(false);

                // get file version that is part of HEAD
                let head_obj = repo.revparse_single("HEAD").unwrap();
                let head_tree = head_obj.peel(ObjectType::Tree).unwrap();
                let head_file_oid = head_tree
                    .as_tree()
                    .unwrap()
                    .get_path(&Path::new(path.file_name().unwrap()))
                    .unwrap()
                    .id();

                let head_blob = repo.find_blob(head_file_oid).unwrap();
                let head_content = str::from_utf8(head_blob.content()).unwrap();

                return Some(head_content.to_owned())
            }
        }

        None
    }

    fn update_diff(&mut self, view: &mut View<ChunkCache>, interval: Interval) {
        if let Some(head_content) = self.get_head_content(view) {
            let mut inserted_spans: Vec<DataSpan> = Vec::new();
            let mut deleted_spans: Vec<DataSpan> = Vec::new();
            let mut modified_spans: Vec<DataSpan> = Vec::new();

            let mut line = 0;
            let mut start_line: Option<usize> = None;
            let mut change: Option<ChangeType> = None;
            let new_content = view.get_document().unwrap();
            let mut diffs = diff::lines(&head_content, &new_content);

            // end of file (manually added)
            diffs.push(diff::Result::Both("", ""));

            // convert deletions and insertions to spans
            for diff in diffs {
                match diff {
                    diff::Result::Both(_, _) => {
                        if let Some(start) = start_line {
                            match change {
                                Some(ChangeType::Insertion) => {
                                    inserted_spans.push(DataSpan {
                                        start: view.offset_of_line(start).unwrap(),
                                        end: view.offset_of_line(line).unwrap_or(view.get_buf_size()) - 1,
                                        data: json!(null),
                                    })
                                }
                                Some(ChangeType::Modification) => {
                                    modified_spans.push(DataSpan {
                                        start: view.offset_of_line(start).unwrap(),
                                        end: view.offset_of_line(line).unwrap_or(view.get_buf_size()) - 1,
                                        data: json!(null),
                                    })
                                }
                                Some(ChangeType::Deletion) => {
                                    deleted_spans.push(DataSpan {
                                        start: view.offset_of_line(start).unwrap(),
                                        end: view.offset_of_line(start).unwrap(),
                                        data: json!(null),
                                    })
                                }
                                _ => {}
                            }

                            start_line = None;
                            change = None;
                        }

                        line += 1;
                    }
                    diff::Result::Right(_) => {
                        if start_line.is_none() {
                            start_line = Some(line);
                            change = Some(ChangeType::Insertion);
                        } else if change != Some(ChangeType::Insertion) {
                            change = Some(ChangeType::Modification);
                        }

                        line += 1;
                    }
                    diff::Result::Left(_) => {
                        if start_line.is_none() {
                            start_line = Some(line);
                            change = Some(ChangeType::Deletion);
                        } else if change != Some(ChangeType::Deletion) {
                            change = Some(ChangeType::Modification);
                        }
                    }
                }
            }

            view.update_annotations(
                interval.start(),
                interval.end(),
                &inserted_spans,
                &AnnotationType::Other("added".to_string()),
            );

            view.update_annotations(
                interval.start(),
                interval.end(),
                &modified_spans,
                &AnnotationType::Other("modified".to_string()),
            );

            view.update_annotations(
                interval.start(),
                interval.end(),
                &deleted_spans,
                &AnnotationType::Other("deleted".to_string()),
            );
        }
    }
}

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
