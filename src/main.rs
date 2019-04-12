//! A plugin to show git diff in xi-editor.
extern crate difference;
extern crate git2;
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;

use git2::{DiffOptions, ObjectType, Repository};
use std::path::Path;

use crate::xi_core::annotations::AnnotationType;
use crate::xi_core::plugin_rpc::DataSpan;
use crate::xi_core::ConfigTable;
use difference::{Changeset, Difference};
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

    fn update_diff(&mut self, view: &mut View<ChunkCache>, interval: Interval) {
        if let Some(path) = view.get_path() {
            if let Ok(repo) = Repository::open(path.parent().unwrap()) {
                let mut opts = DiffOptions::new();
                opts.new_prefix(path)
                    .include_ignored(false)
                    .include_untracked(false);

                if let Ok(odb) = repo.odb() {
                    let obj = repo.revparse_single("HEAD").unwrap();
                    let head_tree = obj.peel(ObjectType::Tree).unwrap();
                    let head_file_oid = head_tree
                        .as_tree()
                        .unwrap()
                        .get_path(&Path::new("test.txt"))
                        .unwrap()
                        .id();
                    let new_oid = odb
                        .write(
                            ObjectType::Blob,
                            view.get_document().unwrap().clone().as_bytes(),
                        )
                        .unwrap();

                    let old = repo.find_blob(new_oid).unwrap();
                    let new = repo.find_blob(head_file_oid).unwrap();

                    let new_content = str::from_utf8(old.content()).unwrap();
                    let old_content = str::from_utf8(new.content()).unwrap();

                    let mut inserted_spans: Vec<DataSpan> = Vec::new();
                    let mut deleted_spans: Vec<DataSpan> = Vec::new();
                    let mut modified_spans: Vec<DataSpan> = Vec::new();

                    let Changeset { diffs, .. } = Changeset::new(old_content, new_content, "\n");

                    let mut offset = 0;
                    let mut start: Option<usize> = None;
                    let mut change: Option<ChangeType> = None;

                    for diff in diffs {
                        match diff {
                            Difference::Same(ref x) => {
                                if let Some(span_start) = start {
                                    match change {
                                        Some(ChangeType::Insertion) => {
                                            inserted_spans.push(DataSpan {
                                                start: span_start,
                                                end: offset,
                                                data: json!(null),
                                            })
                                        }
                                        Some(ChangeType::Modification) => {
                                            modified_spans.push(DataSpan {
                                                start: span_start,
                                                end: offset,
                                                data: json!(null),
                                            })
                                        }
                                        Some(ChangeType::Deletion) => {
                                            deleted_spans.push(DataSpan {
                                                start: span_start,
                                                end: span_start + 1,
                                                data: json!(null),
                                            })
                                        }
                                        _ => {}
                                    }

                                    start = None;
                                    change = None;
                                }

                                offset += x.len() + 1;
                            }
                            Difference::Add(ref x) => {
                                if start.is_none() {
                                    start = Some(offset);
                                    change = Some(ChangeType::Insertion);
                                } else if change != Some(ChangeType::Insertion) {
                                    change = Some(ChangeType::Modification);
                                }

                                offset += x.len() + 1;
                            }
                            Difference::Rem(ref _x) => {
                                if start.is_none() {
                                    start = Some(offset);
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
                        &deleted_spans,
                        &AnnotationType::Other("deleted".to_string()),
                    );

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
                }
            }
        }
    }
}

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
