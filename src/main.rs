//! A plugin to show git diff in xi-editor.
extern crate git2;
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;
extern crate difference;

use git2::{DiffOptions, Repository, ObjectType, Object, Error};
use std::path::Path;

use crate::xi_core::annotations::AnnotationType;
use crate::xi_core::plugin_rpc::DataSpan;
use crate::xi_core::ConfigTable;
use git2::{DiffHunk, Diff, DiffLine};
use serde_json::json;
use xi_plugin_lib::{mainloop, ChunkCache, Plugin, View};
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;
use xi_rope::delta::DeltaRegion;
use std::str;
use difference::{Difference, Changeset};
use serde::ser::{self, Serialize, Serializer};
use serde_json::{self, Value};


struct XiffPlugin {
    inserted_spans: Vec<DataSpan>,
    deleted_spans: Vec<DataSpan>,
    modified_spans: Vec<DataSpan>,

    /// Cached inserts and deletes
    cached: (usize, usize),
}

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

    fn did_save(&mut self, _view: &mut View<Self::Cache>, _old: Option<&Path>) {
//        self.unsaved_inserts = Vec::new();
//        self.unsaved_deletions = Vec::new();
    }

    fn config_changed(&mut self, _view: &mut View<Self::Cache>, _changes: &ConfigTable) {}

    fn update(
        &mut self,
        view: &mut View<Self::Cache>,
        delta: Option<&RopeDelta>,
        _edit_type: String,
        _author: String,
    ) {
//        if let Some(delta) = delta {
//            let (iv, _) = delta.summary();
//
//            let (intersecting_modified, modified): (Vec<DataSpan>, Vec<DataSpan>) = self.modified_spans.clone().into_iter().partition(|s| s.end >= iv.start() && iv.end() >= s.start);
//            let (intersecting_inserted, inserted): (Vec<DataSpan>, Vec<DataSpan>) = self.inserted_spans.clone().into_iter().partition(|s| s.end >= iv.start() && iv.end() >= s.start);
//            let (intersecting_deleted, deleted): (Vec<DataSpan>, Vec<DataSpan>) = self.deleted_spans.clone().into_iter().partition(|s| s.end >= iv.start() && iv.end() >= s.start);
//
//            let modified_to_keep = intersecting_modified.iter().filter(|&s| s.start < iv.start() || s.end > iv.end());
//            let inserted_to_keep = intersecting_modified.iter().filter(|&s| s.start < iv.start() || s.end > iv.end());
//            let deleted_to_keep = intersecting_modified.iter().filter(|&s| s.start < iv.start() || s.end > iv.end());
//
//            for DeltaRegion { old_offset, len, .. } in delta.iter_deletions() {
//                if modified_to_keep.is_empty() && inserted_to_keep.is_empty() {
//
//                }
//            }
//
//            for DeltaRegion { new_offset, len, .. } in delta.iter_inserts() {
//                let line = view.line_of_offset(new_offset).unwrap();
//
////                if !self.unsaved_deletions.contains(&line) {
////                    self.unsaved_inserts.push(line);
////                }
//            }
//        }

        self.update_diff(view, Interval::new(0, view.get_buf_size()));
    }
}

impl XiffPlugin {
    fn new() -> XiffPlugin {
        XiffPlugin {
            inserted_spans: Vec::new(),
            deleted_spans: Vec::new(),
            modified_spans: Vec::new(),
            cached: (0, 0)
        }
    }


    fn diff_changed(&self, diff: &Diff) -> bool {
        if let Ok(stats) = diff.stats() {
            return (stats.insertions(), stats.deletions()) != self.cached
        }

        true
    }

    fn lines_to_data_span(&self, lines: Vec<usize>, view: &mut View<ChunkCache>, deleted: bool) -> Vec<DataSpan> {
        let mut i = 0;
        let mut spans: Vec<DataSpan> = Vec::new();
        let mut sorted_lines = lines.clone();
        sorted_lines.sort();

        while i < sorted_lines.len() {
            let total_deleted = i;
            let start = sorted_lines.get(i).unwrap();

            while i < sorted_lines.len() - 1 && sorted_lines.get(i + 1).unwrap() - 1 <= *sorted_lines.get(i).unwrap() {
                i = i + 1;
            }

            let (start, end) = match deleted {
                true => (view.offset_of_line(*start - total_deleted).unwrap(), view.offset_of_line(*start - total_deleted).unwrap()),
                false => (view.offset_of_line(*start).unwrap(), view.offset_of_line(*sorted_lines.get(i).unwrap() + 1).unwrap() - 1)
            };

            spans.push(DataSpan {
                start,
                end,
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

                if let Ok(odb) = repo.odb() {
                    let obj = repo.revparse_single("HEAD").unwrap();
                    let head_tree = obj.peel(ObjectType::Tree).unwrap();
                    let head_file_oid = head_tree.as_tree().unwrap().get_path(&Path::new("test.txt")).unwrap().id();
                    let new_oid = odb.write(ObjectType::Blob, view.get_document().unwrap().clone().as_bytes()).unwrap();

                    let old = repo.find_blob(new_oid).unwrap();
                    let new = repo.find_blob(head_file_oid).unwrap();

                    let new_content = str::from_utf8(old.content()).unwrap();
                    let old_content = str::from_utf8(new.content()).unwrap();

                    let mut inserted_spans: Vec<DataSpan> = Vec::new();
                    let mut deleted_spans: Vec<DataSpan> = Vec::new();
                    let mut modified_spans: Vec<DataSpan> = Vec::new();

                    eprintln!("old {:?}", old_content);
                    eprintln!("new {:?}", new_content);

                    let Changeset { diffs, .. } = Changeset::new(old_content, new_content, "\n");

                    eprintln!("diff {:?}", diffs);

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
                                                data: json!(null)
                                            })
                                        },
                                        Some(ChangeType::Modification) =>
                                            modified_spans.push(DataSpan {
                                                start: span_start,
                                                end: offset,
                                                data: json!(null)
                                            }),
                                        Some(ChangeType::Deletion) =>
                                            deleted_spans.push(DataSpan {
                                                start: span_start,
                                                end:span_start,
                                                data: json!(null)
                                            }),
                                        _ => {}
                                    }

                                    start = None;
                                    change = None;
                                }

                                offset += x.len() + 1;
                            },
                            Difference::Add(ref x) => {
                                if start.is_none() {
                                    start = Some(offset);
                                    change = Some(ChangeType::Insertion);
                                } else if change != Some(ChangeType::Insertion) {
                                    change = Some(ChangeType::Modification);
                                }

                                offset += x.len() + 1;
                            },
                            Difference::Rem(ref x) => {
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
}

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
