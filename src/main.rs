//! A plugin to show git diff in xi-editor.
extern crate diff;
extern crate git2;
extern crate xi_core_lib as xi_core;
extern crate xi_plugin_lib;
extern crate xi_rope;

use git2::{DiffOptions, ObjectType, Repository, RepositoryOpenFlags};
use std::path::Path;

use crate::xi_core::annotations::AnnotationType;
use crate::xi_core::plugin_rpc::DataSpan;
use crate::xi_core::ConfigTable;
use serde_json;
use serde_json::json;
use std::cmp::max;
use std::ffi::OsStr;
use std::fs;
use std::str;
use xi_plugin_lib::{mainloop, ChunkCache, Plugin, View};
use xi_rope::interval::Interval;
use xi_rope::rope::RopeDelta;

struct XiffPlugin {
    status_item_visible: bool,
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
        view.schedule_idle();

        if let Some(branch) = self.get_current_branch(view) {
            self.status_item_visible = true;
            view.add_status_item("branch", &format!("branch: {}", branch), "left");
        }
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

        if let Some(branch) = self.get_current_branch(view) {
            self.status_item_visible = true;
            view.update_status_item("branch", &format!("branch: {}", branch));
        } else if self.status_item_visible {
            // this prevents the plugin from trying to remove the branch every time when the current
            // directory does not have a git repo
            self.status_item_visible = false;
            view.remove_status_item("branch");
        }
    }
}

impl XiffPlugin {
    fn new() -> XiffPlugin {
        XiffPlugin {
            status_item_visible: false,
        }
    }

    fn get_current_branch(&self, view: &mut View<ChunkCache>) -> Option<String> {
        if let Some(path) = view.get_path() {
            // get repository if current folder is part of one
            if let Ok(repo) = Repository::open_ext(
                path.parent().unwrap(),
                RepositoryOpenFlags::empty(),
                &[] as &[&OsStr],
            ) {
                let head = match repo.head() {
                    Ok(head) => Some(head),
                    Err(_e) => return None,
                };
                let head = head.as_ref().and_then(|h| h.shorthand());

                match head {
                    Some(s) => return Some(s.to_owned()),
                    _ => return None,
                }
            }
        }

        None
    }

    fn get_head_content(&mut self, view: &mut View<ChunkCache>) -> Option<String> {
        if let Some(path) = view.get_path() {
            // get repository if current folder is part of one
            if let Ok(repo) = Repository::open_ext(
                path.parent().unwrap(),
                RepositoryOpenFlags::empty(),
                &[] as &[&OsStr],
            ) {
                let mut opts = DiffOptions::new();
                opts.new_prefix(path)
                    .include_ignored(false)
                    .include_untracked(false);

                // get file version that is part of HEAD
                let head_obj = repo.revparse_single("HEAD").unwrap();
                let head_tree = head_obj.peel(ObjectType::Tree).unwrap();

                // get relative file path of file starting from .git directory
                let parent_path = &repo.path().to_str().unwrap().replace(".git/", "");
                let file_path = fs::canonicalize(&path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace(parent_path, "");

                if let Ok(head_file) = head_tree
                    .as_tree()
                    .unwrap()
                    .get_path(&Path::new(&file_path))
                {
                    let head_file_oid = head_file.id();

                    let head_blob = repo.find_blob(head_file_oid).unwrap();
                    let head_content = str::from_utf8(head_blob.content()).unwrap();

                    return Some(head_content.to_owned());
                }
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
                            let line_end = match view.offset_of_line(line) {
                                Ok(offset) => offset - 1,
                                Err(_) => view.get_buf_size(),
                            };

                            let line_start = view.offset_of_line(start).unwrap();

                            match change {
                                Some(ChangeType::Insertion) => inserted_spans.push(DataSpan {
                                    start: line_start,
                                    end: max(line_end, line_start),
                                    data: json!(null),
                                }),
                                Some(ChangeType::Modification) => modified_spans.push(DataSpan {
                                    start: line_start,
                                    end: max(line_end, line_start),
                                    data: json!(null),
                                }),
                                Some(ChangeType::Deletion) => deleted_spans.push(DataSpan {
                                    start: line_start,
                                    end: line_start,
                                    data: json!(null),
                                }),
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

fn main() {
    let mut plugin = XiffPlugin::new();
    mainloop(&mut plugin).unwrap();
}
