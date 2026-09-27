/* ==========================================================================
 * Transfer planning
 * Turns a request into a plan: every path validated, every destination
 * conflict resolved, every file counted, and nothing created, moved, or
 * deleted. Planning is also the pre-flight gate: unsafe source/destination
 * relationships, unreadable sources, missing destinations, unwritable
 * destinations, and impossible size requests are all refused here, before a
 * job is queued rather than after it has already copied half a tree.
 *
 * The same planning pass serves the dry-run preview the UI shows and the plan
 * the engine executes, so what the user is told is exactly what will happen.
 * ========================================================================== */

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::errors::{AppError, AppResult};
use crate::filesystem::path::normalize;
use crate::platform::paths as platform_paths;
use crate::platform::{self, drives};
use crate::transfer::conflict::{self, Resolution};
use crate::transfer::model::{
    ConflictStrategy, ItemAction, ItemKind, PlanRoot, TransferIssueReason, TransferItem,
    TransferOperation, TransferPlan, TransferRequest,
};
use crate::transfer::safety::{self, SourceKind};
use crate::verification::VerificationPolicy;

/// Upper bound on the entries one transfer plans. A request larger than this is
/// refused up front: planning is exhaustive by design (the engine reports exact
/// totals), and an unbounded plan would let one folder selection exhaust memory.
pub const MAX_PLAN_ITEMS: usize = 100_000;

/// Which planning pass is being made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanMode {
    /// Read-only. Used by the preview the UI shows before the user commits.
    Preview,
    /// Adds the write probe an executable job requires.
    Start,
}

/// One child of a directory being enumerated.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Child {
    name: String,
    path: PathBuf,
    kind: SourceKind,
    size_bytes: u64,
    /// Why the child is not transferable, when it is not.
    note: Option<String>,
}

/// Plans a transfer without touching the filesystem. Used for the preview.
pub fn plan_request(request: &TransferRequest) -> AppResult<TransferPlan> {
    plan_with_limit(request, PlanMode::Preview, MAX_PLAN_ITEMS)
}

/// Plans a transfer that is about to be queued. Adds the destination write
/// probe, so a read-only destination is refused before a job exists.
pub fn plan_for_start(request: &TransferRequest) -> AppResult<TransferPlan> {
    plan_with_limit(request, PlanMode::Start, MAX_PLAN_ITEMS)
}

/// Plans a request with an explicit item budget. Split out so the limit itself
/// is testable without creating a hundred thousand files.
fn plan_with_limit(
    request: &TransferRequest,
    mode: PlanMode,
    max_items: usize,
) -> AppResult<TransferPlan> {
    let destination = safety::validate_destination(&request.destination, mode == PlanMode::Start)?;
    let sources = unique_sources(&request.sources)?;

    let mut planner = Planner {
        destination,
        operation: request.operation,
        strategy: request.conflict,
        verification: request.verification_policy(),
        reserved: HashSet::new(),
        items: Vec::new(),
        roots: Vec::new(),
        max_items,
    };

    for source in &sources {
        planner.add_root(source)?;
    }

    let plan = planner.finish();

    if plan.items.is_empty() {
        return Err(AppError::InvalidInput("nothing to transfer".to_string()));
    }
    ensure_space(&plan, drives::available_bytes(&plan.destination))?;

    Ok(plan)
}

/// Normalized, de-duplicated sources, in the order the user selected them.
fn unique_sources(sources: &[String]) -> AppResult<Vec<String>> {
    if sources.is_empty() {
        return Err(AppError::InvalidInput(
            "a transfer needs at least one source".to_string(),
        ));
    }

    let mut unique: Vec<PathBuf> = Vec::new();
    for source in sources {
        let path = normalize(source)?;
        if unique
            .iter()
            .any(|existing| platform_paths::same_path(existing, &path))
        {
            continue;
        }
        unique.push(path);
    }

    Ok(unique
        .into_iter()
        .map(|path| path.display().to_string())
        .collect())
}

/// Whether the destination can hold the plan.
///
/// A same-volume move is a rename: it needs no free space at all, so a full
/// destination is not a reason to refuse one. `None` from the platform means
/// "unknown", and an unknown amount of space is never treated as zero.
fn ensure_space(plan: &TransferPlan, available_bytes: Option<u64>) -> AppResult<()> {
    if plan.required_bytes == 0 {
        return Ok(());
    }

    let needs_space = plan.operation == TransferOperation::Copy
        || plan
            .roots
            .iter()
            .any(|root| !drives::same_volume(&root.source, &plan.destination));
    if !needs_space {
        return Ok(());
    }

    match available_bytes {
        Some(available) if plan.required_bytes > available => {
            Err(AppError::NotEnoughSpace(format!(
                "this transfer needs {} bytes but '{}' has {} bytes free",
                plan.required_bytes,
                plan.destination.display(),
                available
            )))
        }
        _ => Ok(()),
    }
}

/// Accumulates items and roots for one request. Holds no filesystem state of
/// its own beyond the destination path.
struct Planner {
    destination: PathBuf,
    operation: TransferOperation,
    strategy: ConflictStrategy,
    /// Resolved once from the request, so every item in the plan is verified
    /// the same way.
    verification: VerificationPolicy,
    /// Destination paths already claimed by this plan, so two sources with the
    /// same name can never be planned onto each other.
    reserved: HashSet<PathBuf>,
    items: Vec<TransferItem>,
    roots: Vec<PlanRoot>,
    max_items: usize,
}

/// How one top-level source ended up in the plan.
///
/// A root either transfers whole or is left alone, and a skip reason only
/// exists in the second case. Pairing the two in one value means a transferring
/// root cannot carry a skipped reason, which is what the previous
/// `action` + `skip_reason` + `skip_detail` argument triple allowed.
#[derive(Debug, Clone, Copy)]
enum RootOutcome {
    /// Every item under the root is copied or moved.
    Transfer,
    /// The root is left alone, and this is what is reported about it.
    Skip {
        reason: TransferIssueReason,
        detail: &'static str,
    },
}

impl RootOutcome {
    fn action(self) -> ItemAction {
        match self {
            Self::Transfer => ItemAction::Transfer,
            Self::Skip { .. } => ItemAction::Skip,
        }
    }
}

impl Planner {
    /// Plans one top-level source.
    fn add_root(&mut self, source: &str) -> AppResult<()> {
        let facts = safety::inspect_source(source)?;
        let destination = self.destination.join(facts.name());

        // The relationship is checked before anything is enumerated, so a
        // folder cannot start copying into itself and be rejected halfway.
        safety::ensure_safe_relationship(&facts.path, &destination, facts.kind, self.strategy)?;

        let root_index = self.roots.len();
        let item_start = self.items.len();

        if facts.kind == SourceKind::Unsupported {
            let existing = conflict::existing_kind(&destination);
            self.push_skipped(
                &facts.path,
                &destination,
                ItemKind::File,
                0,
                root_index,
                TransferIssueReason::Unsupported,
                UNSUPPORTED_DETAIL,
                existing,
            );
            self.push_root(
                facts.path,
                destination,
                ItemKind::File,
                RootOutcome::Skip {
                    reason: TransferIssueReason::Unsupported,
                    detail: UNSUPPORTED_DETAIL,
                },
                item_start,
            );
            return Ok(());
        }

        let kind = item_kind(facts.kind);
        let existing = conflict::existing_kind(&destination);

        match conflict::resolve(&destination, self.strategy, &self.reserved, existing)? {
            Resolution::Skip => {
                self.push_skipped(
                    &facts.path,
                    &destination,
                    kind,
                    facts.size_bytes,
                    root_index,
                    TransferIssueReason::Skipped,
                    SKIP_DETAIL,
                    existing,
                );
                self.push_root(
                    facts.path,
                    destination,
                    kind,
                    RootOutcome::Skip {
                        reason: TransferIssueReason::Skipped,
                        detail: SKIP_DETAIL,
                    },
                    item_start,
                );
                return Ok(());
            }
            Resolution::Write(resolved) => {
                self.reserved.insert(conflict::collision_key(&resolved));
                match facts.kind {
                    SourceKind::File => {
                        self.push_file(
                            &facts.path,
                            &resolved,
                            facts.size_bytes,
                            root_index,
                            existing,
                        );
                    }
                    SourceKind::Directory => {
                        self.push_directory(&facts.path, &resolved, root_index, existing);
                        self.enumerate(&facts.path, &resolved, root_index)?;
                    }
                    SourceKind::Unsupported => unreachable!("handled above"),
                }
            }
        }

        self.push_root(
            facts.path,
            destination,
            kind,
            RootOutcome::Transfer,
            item_start,
        );
        Ok(())
    }

    /// Records a root and the items it produced.
    fn push_root(
        &mut self,
        source: PathBuf,
        destination: PathBuf,
        kind: ItemKind,
        outcome: RootOutcome,
        item_start: usize,
    ) {
        let action = outcome.action();
        let items = &self.items[item_start..];
        let files = items.iter().filter(|item| item.transfers_bytes()).count() as u64;
        let bytes = items
            .iter()
            .filter(|item| item.transfers_bytes())
            .map(|item| item.size_bytes)
            .sum();
        let skipped_bytes = items
            .iter()
            .filter(|item| item.action == ItemAction::Skip && item.kind == ItemKind::File)
            .map(|item| item.size_bytes)
            .sum();
        // A root is "clean" when nothing inside it collides with an existing
        // destination entry. That is the condition under which a same-volume
        // move can rename the whole tree in one atomic step.
        let clean = action == ItemAction::Transfer && items.iter().all(|item| !item.existed);

        self.roots.push(PlanRoot {
            source,
            destination,
            kind,
            action,
            skip_reason: match outcome {
                RootOutcome::Skip { reason, .. } => Some(reason),
                RootOutcome::Transfer => None,
            },
            skip_detail: match outcome {
                RootOutcome::Skip { detail, .. } => Some(detail.to_string()),
                RootOutcome::Transfer => None,
            },
            item_start,
            item_count: items.len(),
            files,
            bytes,
            skipped_bytes,
            clean,
        });
    }

    fn push_file(
        &mut self,
        source: &Path,
        destination: &Path,
        size_bytes: u64,
        root_index: usize,
        existing: Option<ItemKind>,
    ) {
        self.items.push(TransferItem {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            kind: ItemKind::File,
            size_bytes,
            root_index,
            action: ItemAction::Transfer,
            skip_reason: None,
            skip_detail: None,
            // A file landing where a directory sits can only be written after
            // that directory is gone, which only `Replace` asks for.
            clear_first: (existing == Some(ItemKind::Directory)).then_some(ItemKind::Directory),
            existed: existing.is_some(),
        });
    }

    fn push_directory(
        &mut self,
        source: &Path,
        destination: &Path,
        root_index: usize,
        existing: Option<ItemKind>,
    ) {
        self.items.push(TransferItem {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            kind: ItemKind::Directory,
            size_bytes: 0,
            root_index,
            action: ItemAction::Transfer,
            skip_reason: None,
            skip_detail: None,
            clear_first: (existing == Some(ItemKind::File)).then_some(ItemKind::File),
            existed: existing.is_some(),
        });
    }

    /// Records an item the engine will not transfer. `existing` is passed in
    /// when the caller already inspected the destination.
    #[allow(clippy::too_many_arguments)]
    fn push_skipped(
        &mut self,
        source: &Path,
        destination: &Path,
        kind: ItemKind,
        size_bytes: u64,
        root_index: usize,
        reason: TransferIssueReason,
        detail: &str,
        existing: Option<ItemKind>,
    ) {
        self.items.push(TransferItem {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            kind,
            size_bytes,
            root_index,
            action: ItemAction::Skip,
            skip_reason: Some(reason),
            skip_detail: Some(detail.to_string()),
            clear_first: None,
            existed: existing.is_some(),
        });
    }

    /// Walks a source folder, planning every entry it can transfer and
    /// reporting every entry it will not.
    ///
    /// Iterative rather than recursive: the depth of a source tree belongs to
    /// the user's disk, not to this process's stack.
    fn enumerate(
        &mut self,
        source_root: &Path,
        destination_root: &Path,
        root_index: usize,
    ) -> AppResult<()> {
        let mut pending: Vec<(PathBuf, PathBuf)> =
            vec![(source_root.to_path_buf(), destination_root.to_path_buf())];

        while let Some((source_dir, destination_dir)) = pending.pop() {
            let children = match read_children(&source_dir) {
                Ok(children) => children,
                Err(error) => {
                    if platform_paths::same_path(&source_dir, source_root) {
                        // The folder the user picked cannot be read at all:
                        // refuse the request instead of planning half of it.
                        return Err(error);
                    }
                    // A nested folder that cannot be read is reported and left
                    // behind; the rest of the transfer still makes sense.
                    self.push_skipped(
                        &source_dir,
                        &destination_dir,
                        ItemKind::Directory,
                        0,
                        root_index,
                        TransferIssueReason::Unsupported,
                        &format!("folder was not read: {error}"),
                        None,
                    );
                    continue;
                }
            };

            let mut child_directories: Vec<(PathBuf, PathBuf)> = Vec::new();

            for child in children {
                if self.items.len() >= self.max_items {
                    return Err(AppError::TooManyItems(format!(
                        "'{}' expands into more than {} entries",
                        source_root.display(),
                        self.max_items
                    )));
                }

                let destination = destination_dir.join(&child.name);

                if child.kind == SourceKind::Unsupported {
                    let existing = conflict::existing_kind(&destination);
                    self.push_skipped(
                        &child.path,
                        &destination,
                        ItemKind::File,
                        child.size_bytes,
                        root_index,
                        TransferIssueReason::Unsupported,
                        child.note.as_deref().unwrap_or(UNSUPPORTED_DETAIL),
                        existing,
                    );
                    continue;
                }

                let existing = conflict::existing_kind(&destination);
                match conflict::resolve(&destination, self.strategy, &self.reserved, existing)? {
                    Resolution::Skip => {
                        // A skipped folder is never walked: its contents stay
                        // exactly as they are, which is what skip means.
                        self.push_skipped(
                            &child.path,
                            &destination,
                            item_kind(child.kind),
                            child.size_bytes,
                            root_index,
                            TransferIssueReason::Skipped,
                            SKIP_DETAIL,
                            existing,
                        );
                    }
                    Resolution::Write(resolved) => {
                        self.reserved.insert(conflict::collision_key(&resolved));
                        match child.kind {
                            SourceKind::File => self.push_file(
                                &child.path,
                                &resolved,
                                child.size_bytes,
                                root_index,
                                existing,
                            ),
                            SourceKind::Directory => {
                                self.push_directory(&child.path, &resolved, root_index, existing);
                                child_directories.push((child.path, resolved));
                            }
                            SourceKind::Unsupported => unreachable!("handled above"),
                        }
                    }
                }
            }

            // Reverse order keeps the pop order deterministic: directories
            // before files, names ascending, parents before children.
            for entry in child_directories.into_iter().rev() {
                pending.push(entry);
            }
        }

        Ok(())
    }

    /// Derives every total from the planned items, so the reported numbers and
    /// the work to be done cannot drift apart.
    fn finish(self) -> TransferPlan {
        let counted: Vec<&TransferItem> = self
            .items
            .iter()
            .filter(|item| item.transfers_bytes())
            .collect();

        TransferPlan {
            destination: self.destination,
            operation: self.operation,
            conflict: self.strategy,
            verification: self.verification,
            roots: self.roots,
            total_bytes: counted.iter().map(|item| item.size_bytes).sum(),
            total_files: counted.len() as u64,
            total_directories: self
                .items
                .iter()
                .filter(|item| {
                    item.action == ItemAction::Transfer && item.kind == ItemKind::Directory
                })
                .count() as u64,
            conflicts: self.items.iter().filter(|item| item.existed).count() as u64,
            skipped_items: self
                .items
                .iter()
                .filter(|item| item.action == ItemAction::Skip)
                .count() as u64,
            skipped_bytes: self
                .items
                .iter()
                .filter(|item| item.action == ItemAction::Skip)
                .map(|item| item.size_bytes)
                .sum(),
            required_bytes: counted.iter().map(|item| item.size_bytes).sum(),
            items: self.items,
        }
    }
}

/// Reads one directory, describing each entry from its own metadata so links
/// and reparse points are never followed.
fn read_children(directory: &Path) -> AppResult<Vec<Child>> {
    let entries =
        std::fs::read_dir(directory).map_err(|error| safety::read_error(error, directory))?;

    let mut children: Vec<Child> = Vec::new();
    for entry in entries {
        // An entry that vanished between the read and its description is
        // normal system behaviour, not a reason to fail the walk.
        let Ok(entry) = entry else {
            continue;
        };

        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();

        let (kind, size_bytes, note) = match entry.metadata() {
            Ok(metadata) => {
                if platform::is_reparse_point(&metadata) {
                    (
                        SourceKind::Unsupported,
                        0,
                        Some(UNSUPPORTED_DETAIL.to_string()),
                    )
                } else if metadata.is_dir() {
                    (SourceKind::Directory, 0, None)
                } else if metadata.is_file() {
                    (SourceKind::File, metadata.len(), None)
                } else {
                    (
                        SourceKind::Unsupported,
                        0,
                        Some("special filesystem entry".to_string()),
                    )
                }
            }
            Err(error) => (
                SourceKind::Unsupported,
                0,
                Some(format!("entry could not be inspected: {error}")),
            ),
        };

        children.push(Child {
            name,
            path,
            kind,
            size_bytes,
            note,
        });
    }

    sort_children(&mut children);
    Ok(children)
}

/// Deterministic order: folders first, then files, then entries that will not
/// be transferred; case-insensitive by name with the exact name as a
/// tie-breaker, matching the directory browser.
fn sort_children(children: &mut [Child]) {
    fn rank(kind: SourceKind) -> u8 {
        match kind {
            SourceKind::Directory => 0,
            SourceKind::File => 1,
            SourceKind::Unsupported => 2,
        }
    }

    children.sort_by(|left, right| {
        rank(left.kind)
            .cmp(&rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn item_kind(kind: SourceKind) -> ItemKind {
    match kind {
        SourceKind::Directory => ItemKind::Directory,
        _ => ItemKind::File,
    }
}

const UNSUPPORTED_DETAIL: &str =
    "symbolic links and reparse points are reported but never followed or copied";
const SKIP_DETAIL: &str =
    "an entry already exists at this destination; the skip strategy left it untouched";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    fn copy_request(sources: &[&Path], destination: &Path) -> TransferRequest {
        TransferRequest {
            sources: sources
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            destination: destination.display().to_string(),
            operation: TransferOperation::Copy,
            conflict: ConflictStrategy::Skip,
            verification: None,
        }
    }

    fn write_file(path: &Path, bytes: usize) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent is creatable");
        }
        std::fs::write(path, vec![0u8; bytes]).expect("file is writable");
    }

    fn clean_up(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn names(items: &[TransferItem]) -> Vec<String> {
        items
            .iter()
            .map(|item| {
                format!(
                    "{}{}",
                    crate::filesystem::display_name(&item.destination),
                    if item.action == ItemAction::Skip {
                        " (skip)"
                    } else {
                        ""
                    }
                )
            })
            .collect()
    }

    #[test]
    fn plans_a_single_file_copy() {
        let workspace = unique_temp_dir("plan-single");
        let source = workspace.join("payload.bin");
        write_file(&source, 5_000);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.total_files, 1);
        assert_eq!(plan.total_bytes, 5_000);
        assert_eq!(plan.total_directories, 0);
        assert_eq!(plan.conflicts, 0);
        assert_eq!(plan.skipped_items, 0);
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].kind, ItemKind::File);
        assert_eq!(plan.items[0].destination, destination.join("payload.bin"));
        assert_eq!(plan.roots.len(), 1);
        assert!(
            plan.roots[0].clean,
            "nothing collides, so a move may rename"
        );

        clean_up(&workspace);
    }

    #[test]
    fn plans_a_nested_tree_with_parents_before_children() {
        let workspace = unique_temp_dir("plan-nested");
        let source = workspace.join("Data");
        write_file(&source.join("root.txt"), 10);
        write_file(&source.join("sub").join("inner.txt"), 20);
        write_file(&source.join("sub").join("deeper").join("leaf.bin"), 30);
        std::fs::create_dir_all(source.join("empty")).expect("subdir");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.total_files, 3);
        assert_eq!(plan.total_bytes, 60);
        assert_eq!(
            plan.total_directories, 4,
            "the folder itself, sub, deeper, and empty"
        );
        assert_eq!(
            names(&plan.items),
            vec![
                "Data",
                "empty",
                "sub",
                "root.txt",
                "deeper",
                "inner.txt",
                "leaf.bin"
            ],
            "folders first within a folder, then its children, parents before their own children"
        );

        let root = &plan.roots[0];
        assert_eq!(plan.root_items(root).len(), plan.items.len());
        assert_eq!(root.files, 3);
        assert_eq!(root.bytes, 60);

        clean_up(&workspace);
    }

    #[test]
    fn plans_an_empty_directory() {
        let workspace = unique_temp_dir("plan-empty");
        let source = workspace.join("Empty");
        std::fs::create_dir_all(&source).expect("subdir");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.total_files, 0);
        assert_eq!(plan.total_bytes, 0);
        assert_eq!(plan.total_directories, 1);
        assert_eq!(plan.required_bytes, 0);

        clean_up(&workspace);
    }

    #[test]
    fn plans_multiple_sources_as_one_job() {
        let workspace = unique_temp_dir("plan-multiple");
        let first = workspace.join("a.txt");
        let second = workspace.join("b.txt");
        write_file(&first, 10);
        write_file(&second, 20);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan =
            plan_request(&copy_request(&[&first, &second], &destination)).expect("plan succeeds");

        assert_eq!(plan.roots.len(), 2);
        assert_eq!(plan.total_files, 2);
        assert_eq!(plan.total_bytes, 30);
        assert_eq!(plan.items[0].root_index, 0);
        assert_eq!(plan.items[1].root_index, 1);

        clean_up(&workspace);
    }

    #[test]
    fn duplicate_sources_are_planned_once() {
        let workspace = unique_temp_dir("plan-duplicates");
        let source = workspace.join("a.txt");
        write_file(&source, 10);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan =
            plan_request(&copy_request(&[&source, &source], &destination)).expect("plan succeeds");

        assert_eq!(plan.roots.len(), 1);
        assert_eq!(plan.total_files, 1);

        clean_up(&workspace);
    }

    #[test]
    fn skip_leaves_collisions_alone_and_counts_them() {
        let workspace = unique_temp_dir("plan-skip");
        let source = workspace.join("notes.txt");
        write_file(&source, 10);
        let destination = workspace.join("out");
        write_file(&destination.join("notes.txt"), 99);

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.total_files, 0);
        assert_eq!(plan.conflicts, 1);
        assert_eq!(plan.skipped_items, 1);
        assert_eq!(plan.skipped_bytes, 10);
        assert_eq!(plan.items[0].action, ItemAction::Skip);
        assert_eq!(
            plan.items[0].skip_reason,
            Some(TransferIssueReason::Skipped)
        );
        assert!(
            !plan.roots[0].clean,
            "a colliding root can never take the rename fast path"
        );

        clean_up(&workspace);
    }

    #[test]
    fn replace_plans_the_colliding_path_and_clears_a_type_mismatch() {
        let workspace = unique_temp_dir("plan-replace");
        let source = workspace.join("Data");
        write_file(&source.join("inner.txt"), 5);
        let destination = workspace.join("out");
        // A file sits where the folder must go, and a folder sits where the
        // file inside it must go.
        write_file(&destination.join("Data"), 3);
        std::fs::create_dir_all(destination.join("Data (backup)")).expect("subdir");

        let mut request = copy_request(&[&source], &destination);
        request.conflict = ConflictStrategy::Replace;
        let plan = plan_request(&request).expect("plan succeeds");

        let root_item = &plan.items[0];
        assert_eq!(root_item.destination, destination.join("Data"));
        assert_eq!(root_item.action, ItemAction::Transfer);
        assert_eq!(
            root_item.clear_first,
            Some(ItemKind::File),
            "the blocking file is cleared first"
        );
        assert!(root_item.existed);
        assert_eq!(plan.conflicts, 1);

        clean_up(&workspace);
    }

    #[test]
    fn replace_marks_a_blocking_directory_on_a_file_item() {
        let workspace = unique_temp_dir("plan-replace-dir");
        let source = workspace.join("payload.bin");
        write_file(&source, 8);
        let destination = workspace.join("out");
        std::fs::create_dir_all(destination.join("payload.bin")).expect("subdir");

        let mut request = copy_request(&[&source], &destination);
        request.conflict = ConflictStrategy::Replace;
        let plan = plan_request(&request).expect("plan succeeds");

        assert_eq!(plan.items[0].clear_first, Some(ItemKind::Directory));
        assert!(plan.items[0].existed);

        clean_up(&workspace);
    }

    #[test]
    fn rename_places_a_copy_beside_the_existing_entry() {
        let workspace = unique_temp_dir("plan-rename");
        let source = workspace.join("report.txt");
        write_file(&source, 10);
        let destination = workspace.join("out");
        write_file(&destination.join("report.txt"), 99);
        write_file(&destination.join("report (2).txt"), 99);

        let mut request = copy_request(&[&source], &destination);
        request.conflict = ConflictStrategy::Rename;
        let plan = plan_request(&request).expect("plan succeeds");

        assert_eq!(
            plan.items[0].destination,
            destination.join("report (3).txt"),
            "generated names skip every occupied name"
        );
        assert_eq!(plan.conflicts, 1);
        assert!(!plan.roots[0].clean);

        clean_up(&workspace);
    }

    #[test]
    fn rename_of_a_folder_moves_its_children_under_the_new_name() {
        let workspace = unique_temp_dir("plan-rename-folder");
        let source = workspace.join("Data");
        write_file(&source.join("inner.txt"), 4);
        let destination = workspace.join("out");
        std::fs::create_dir_all(destination.join("Data")).expect("subdir");

        let mut request = copy_request(&[&source], &destination);
        request.conflict = ConflictStrategy::Rename;
        let plan = plan_request(&request).expect("plan succeeds");

        assert_eq!(plan.items[0].destination, destination.join("Data (2)"));
        assert_eq!(
            plan.items[1].destination,
            destination.join("Data (2)").join("inner.txt"),
            "children follow the renamed root"
        );

        clean_up(&workspace);
    }

    #[test]
    fn two_sources_with_the_same_name_never_share_a_destination() {
        let workspace = unique_temp_dir("plan-same-name");
        let first = workspace.join("one").join("report.txt");
        let second = workspace.join("two").join("report.txt");
        write_file(&first, 10);
        write_file(&second, 20);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let mut request = copy_request(&[&first, &second], &destination);
        request.conflict = ConflictStrategy::Rename;
        let plan = plan_request(&request).expect("plan succeeds");

        let destinations: Vec<PathBuf> = plan
            .items
            .iter()
            .map(|item| item.destination.clone())
            .collect();
        assert_eq!(
            destinations,
            vec![
                destination.join("report.txt"),
                destination.join("report (2).txt")
            ],
            "the second source cannot overwrite the first"
        );

        clean_up(&workspace);
    }

    #[test]
    fn unreadable_sources_are_refused_before_the_job_exists() {
        let workspace = unique_temp_dir("plan-missing-source");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let error = plan_request(&copy_request(&[&workspace.join("gone.txt")], &destination))
            .expect_err("a missing source is refused");

        assert_eq!(error.code(), "path_not_found");

        clean_up(&workspace);
    }

    #[test]
    fn a_missing_destination_is_refused() {
        let workspace = unique_temp_dir("plan-missing-destination");
        let source = workspace.join("a.txt");
        write_file(&source, 4);

        let error = plan_request(&copy_request(&[&source], &workspace.join("nowhere")))
            .expect_err("the destination must exist");

        assert_eq!(error.code(), "path_not_found");

        clean_up(&workspace);
    }

    #[test]
    fn a_file_destination_is_refused() {
        let workspace = unique_temp_dir("plan-file-destination");
        let source = workspace.join("a.txt");
        write_file(&source, 4);
        let destination = workspace.join("not-a-folder");
        write_file(&destination, 4);

        let error = plan_request(&copy_request(&[&source], &destination))
            .expect_err("a file is not a destination");

        assert_eq!(error.code(), "path_not_directory");

        clean_up(&workspace);
    }

    #[test]
    fn a_folder_cannot_be_planned_into_itself() {
        let workspace = unique_temp_dir("plan-into-itself");
        let source = workspace.join("Data");
        write_file(&source.join("inner.txt"), 4);
        let destination = source.join("Backup");
        std::fs::create_dir_all(&destination).expect("subdir");

        let error = plan_request(&copy_request(&[&source], &destination))
            .expect_err("copying a folder into itself is refused");

        assert_eq!(error.code(), "unsafe_relationship");
        assert!(
            plan_request(&copy_request(&[&source], &destination)).is_err(),
            "the refusal happens before any enumeration"
        );

        clean_up(&workspace);
    }

    #[test]
    fn copying_a_folder_into_its_own_parent_needs_a_rename_strategy() {
        let workspace = unique_temp_dir("plan-own-parent");
        let parent = workspace.join("parent");
        let source = parent.join("Data");
        write_file(&source.join("inner.txt"), 4);

        let error = plan_request(&copy_request(&[&source], &parent))
            .expect_err("replace would write the folder over itself");
        assert_eq!(error.code(), "unsafe_relationship");

        let mut request = copy_request(&[&source], &parent);
        request.conflict = ConflictStrategy::Rename;
        let plan = plan_request(&request).expect("a renamed sibling copy is allowed");

        assert_eq!(plan.items[0].destination, parent.join("Data (2)"));

        clean_up(&workspace);
    }

    #[test]
    fn links_are_reported_and_never_followed() {
        let workspace = unique_temp_dir("plan-link");
        let source = workspace.join("Data");
        write_file(&source.join("real.txt"), 6);
        let target = workspace.join("outside.txt");
        write_file(&target, 6);
        let link = source.join("link.txt");

        if !try_symlink_file(&target, &link) {
            clean_up(&workspace);
            return;
        }

        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");
        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.total_files, 1, "only the real file is transferred");
        assert_eq!(plan.skipped_items, 1);
        let skipped = plan
            .items
            .iter()
            .find(|item| item.action == ItemAction::Skip)
            .expect("the link is reported");
        assert_eq!(skipped.skip_reason, Some(TransferIssueReason::Unsupported));

        clean_up(&workspace);
    }

    #[test]
    fn a_plan_larger_than_the_budget_is_refused() {
        let workspace = unique_temp_dir("plan-budget");
        let source = workspace.join("Data");
        for index in 0..6 {
            write_file(&source.join(format!("file-{index}.txt")), 1);
        }
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let error = plan_with_limit(
            &copy_request(&[&source], &destination),
            PlanMode::Preview,
            4,
        )
        .expect_err("the budget is enforced");

        assert_eq!(error.code(), "too_many_items");

        clean_up(&workspace);
    }

    #[test]
    fn a_request_without_sources_is_refused() {
        let workspace = unique_temp_dir("plan-no-sources");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let error =
            plan_request(&copy_request(&[], &destination)).expect_err("nothing to transfer");

        assert_eq!(error.code(), "invalid_input");

        clean_up(&workspace);
    }

    #[test]
    fn preview_planning_never_touches_the_destination() {
        let workspace = unique_temp_dir("plan-preview");
        let source = workspace.join("Data");
        write_file(&source.join("inner.txt"), 4);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert!(!plan.destination.join("Data").exists());
        assert!(plan.total_bytes > 0);
        assert!(
            std::fs::read_dir(&destination)
                .expect("readable")
                .next()
                .is_none(),
            "a preview leaves the destination exactly as it was"
        );

        clean_up(&workspace);
    }

    #[test]
    fn starting_a_plan_probes_the_destination_and_cleans_up() {
        let workspace = unique_temp_dir("plan-start");
        let source = workspace.join("a.txt");
        write_file(&source, 4);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        plan_for_start(&copy_request(&[&source], &destination))
            .expect("the destination is writable");

        assert!(
            std::fs::read_dir(&destination)
                .expect("readable")
                .next()
                .is_none(),
            "the write probe removes itself"
        );

        clean_up(&workspace);
    }

    #[test]
    fn space_is_only_required_when_bytes_are_written() {
        let workspace = unique_temp_dir("plan-space");
        let source = workspace.join("big.bin");
        write_file(&source, 1_000);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.required_bytes, 1_000);
        ensure_space(&plan, Some(10_000)).expect("enough space");
        assert_eq!(
            ensure_space(&plan, Some(999))
                .expect_err("not enough space")
                .code(),
            "not_enough_space"
        );
        ensure_space(&plan, None).expect("an unknown amount of space is not zero");

        clean_up(&workspace);
    }

    #[test]
    fn an_empty_directory_transfer_never_needs_space() {
        let workspace = unique_temp_dir("plan-space-empty");
        let source = workspace.join("Empty");
        std::fs::create_dir_all(&source).expect("subdir");
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        ensure_space(&plan, Some(0)).expect("creating a folder needs no free space");

        clean_up(&workspace);
    }

    #[test]
    fn a_same_volume_move_needs_no_free_space() {
        let workspace = unique_temp_dir("plan-space-move");
        let source = workspace.join("Data");
        write_file(&source.join("inner.bin"), 4_000);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let mut request = copy_request(&[&source], &destination);
        request.operation = TransferOperation::Move;
        let plan = plan_request(&request).expect("plan succeeds");

        assert_eq!(plan.operation, TransferOperation::Move);
        assert!(
            plan.roots[0].clean,
            "an uncontested move is a single rename"
        );
        ensure_space(&plan, Some(0)).expect("a rename needs no space");

        clean_up(&workspace);
    }

    #[test]
    fn the_plan_reports_the_paths_the_frontend_shows() {
        let workspace = unique_temp_dir("plan-report");
        let source = workspace.join("a.txt");
        write_file(&source, 12);
        let destination = workspace.join("out");
        std::fs::create_dir_all(&destination).expect("destination is creatable");

        let plan = plan_request(&copy_request(&[&source], &destination)).expect("plan succeeds");

        assert_eq!(plan.destination, destination);
        assert_eq!(plan.operation, TransferOperation::Copy);
        assert_eq!(plan.conflict, ConflictStrategy::Skip);
        assert_eq!(plan.items[0].root_index, 0);

        clean_up(&workspace);
    }

    /// Creates a file symlink, returning `false` when the platform refuses
    /// (Windows without developer mode).
    fn try_symlink_file(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (target, link);
            false
        }
    }
}
