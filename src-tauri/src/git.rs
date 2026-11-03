use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use git2::build::CheckoutBuilder;
use git2::{
    BranchType, DiffFormat, DiffOptions, IndexAddOption, Repository, Status, StatusOptions,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn open(path: &Path) -> Result<Repository, String> {
    Repository::open(path).map_err(|e| e.message().to_string())
}

fn resolve_commit<'a>(repo: &'a Repository, rev: &str) -> Result<git2::Commit<'a>, String> {
    let object = repo
        .revparse_single(rev)
        .map_err(|e| format!("Cannot resolve revision '{}': {}", rev, e.message()))?;
    object
        .peel_to_commit()
        .map_err(|e| format!("Revision '{}' is not a commit: {}", rev, e.message()))
}

fn commit_info(oid: git2::Oid, commit: &git2::Commit<'_>) -> CommitInfo {
    let author = commit.author();
    CommitInfo {
        oid: oid.to_string(),
        short: short(oid),
        summary: commit.summary().unwrap_or("").to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: author.name().unwrap_or("").to_string(),
        email: author.email().unwrap_or("").to_string(),
        time: commit.time().seconds(),
        parents: commit.parent_ids().map(|p| p.to_string()).collect(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    path: String,
    name: String,
    branch: String,
    head: String,
    detached: bool,
    unborn: bool,
    dirty: bool,
    ahead: usize,
    behind: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatus {
    path: String,
    index: String,
    worktree: String,
    staged: bool,
    unstaged: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    oid: String,
    short: String,
    summary: String,
    message: String,
    author: String,
    email: String,
    time: i64,
    parents: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    name: String,
    is_head: bool,
    is_remote: bool,
    merged: bool,
    upstream: Option<String>,
    ahead: usize,
    behind: usize,
    target: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    name: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    name: String,
    target: String,
    message: String,
    annotated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashInfo {
    index: usize,
    message: String,
    oid: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {
    name: String,
    email: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitProgress {
    operation: String,
    line: String,
}

fn short(oid: git2::Oid) -> String {
    let s = oid.to_string();
    s.chars().take(7).collect()
}

fn current_branch(repo: &Repository) -> (String, bool, bool) {
    match repo.head() {
        Ok(head) => {
            if repo.head_detached().unwrap_or(false) {
                ("(detached HEAD)".to_string(), true, false)
            } else {
                (head.shorthand().unwrap_or("HEAD").to_string(), false, false)
            }
        }
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            let name = repo
                .find_reference("HEAD")
                .ok()
                .and_then(|r| r.symbolic_target().map(String::from))
                .and_then(|t| t.strip_prefix("refs/heads/").map(String::from))
                .unwrap_or_else(|| "main".to_string());
            (name, false, true)
        }
        Err(_) => ("(unknown)".to_string(), false, true),
    }
}

fn is_dirty(repo: &Repository) -> bool {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    repo.statuses(Some(&mut opts))
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

fn ahead_behind(repo: &Repository) -> (usize, usize) {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return (0, 0),
    };
    if repo.head_detached().unwrap_or(false) {
        return (0, 0);
    }
    let branch = match repo.find_branch(head.shorthand().unwrap_or(""), BranchType::Local) {
        Ok(b) => b,
        Err(_) => return (0, 0),
    };
    let upstream = match branch.upstream() {
        Ok(u) => u,
        Err(_) => return (0, 0),
    };
    let local = match head.target() {
        Some(o) => o,
        None => return (0, 0),
    };
    let remote = match upstream.get().target() {
        Some(o) => o,
        None => return (0, 0),
    };
    repo.graph_ahead_behind(local, remote).unwrap_or((0, 0))
}

fn repo_info(path: &Path, repo: &Repository, dirty: bool) -> RepoInfo {
    let (branch, detached, unborn) = current_branch(repo);
    let head = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .map(short)
        .unwrap_or_default();
    let (ahead, behind) = ahead_behind(repo);

    RepoInfo {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        branch,
        head,
        detached,
        unborn,
        dirty,
        ahead,
        behind,
    }
}

#[tauri::command(async)]
pub fn open_repository(path: String) -> Result<RepoInfo, String> {
    let p = PathBuf::from(&path);
    let repo = open(&p)?;
    // Worktrees expose the main workdir via workdir().
    let info_path = repo.workdir().unwrap_or(&p).to_path_buf();
    let dirty = is_dirty(&repo);
    Ok(repo_info(&info_path, &repo, dirty))
}

#[tauri::command(async)]
pub fn init_repository(path: String) -> Result<RepoInfo, String> {
    let p = PathBuf::from(&path);
    std::fs::create_dir_all(&p).map_err(err)?;
    let repo = Repository::init(&p).map_err(|e| e.message().to_string())?;
    let dirty = is_dirty(&repo);
    Ok(repo_info(&p, &repo, dirty))
}

#[tauri::command(async)]
pub fn get_repository_info(repository: String) -> Result<RepoInfo, String> {
    let path = PathBuf::from(&repository);
    let repo = open(&path)?;
    let dirty = is_dirty(&repo);
    Ok(repo_info(&path, &repo, dirty))
}

#[tauri::command(async)]
pub fn get_snapshot(
    repository: String,
    log_limit: Option<usize>,
    include_remote: bool,
) -> Result<Snapshot, String> {
    let p = PathBuf::from(&repository);
    let mut repo = open(&p)?;

    let status = status_entries(&repo)?;
    let dirty = !status.is_empty();
    let workdir = repo.workdir().unwrap_or(&p).to_path_buf();
    let info = repo_info(&workdir, &repo, dirty);
    let log = log_entries(&repo, log_limit.unwrap_or(300), 0)?;
    let branches = branch_entries(&repo)?;
    let graph = branch_graph(&repo, include_remote)?;
    let remotes = remote_entries(&repo)?;
    let tags = tag_entries(&repo)?;
    let stashes = stash_entries(&mut repo)?;
    let user = user_config(&repo)?;

    Ok(Snapshot {
        repo: info,
        status,
        log,
        branches,
        graph,
        remotes,
        tags,
        stashes,
        user,
    })
}

#[tauri::command(async)]
pub fn clone_repository(
    app: AppHandle,
    url: String,
    destination: String,
) -> Result<RepoInfo, String> {
    let dest = PathBuf::from(&destination);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(err)?;
    }
    run_git_streaming(
        Path::new("."),
        &["clone", "--progress", &url, &destination],
        &app,
        "clone",
    )?;
    let repo = open(&dest)?;
    let dirty = is_dirty(&repo);
    Ok(repo_info(&dest, &repo, dirty))
}

fn status_letters(s: Status) -> (String, bool, bool) {
    if s.contains(Status::CONFLICTED) {
        return ("U".to_string(), true, true);
    }
    let idx = if s.contains(Status::INDEX_NEW) {
        "A"
    } else if s.contains(Status::INDEX_MODIFIED) {
        "M"
    } else if s.contains(Status::INDEX_DELETED) {
        "D"
    } else if s.contains(Status::INDEX_RENAMED) {
        "R"
    } else if s.contains(Status::INDEX_TYPECHANGE) {
        "T"
    } else {
        " "
    };
    let wt = if s.contains(Status::WT_NEW) {
        "?"
    } else if s.contains(Status::WT_MODIFIED) {
        "M"
    } else if s.contains(Status::WT_DELETED) {
        "D"
    } else if s.contains(Status::WT_RENAMED) {
        "R"
    } else if s.contains(Status::WT_TYPECHANGE) {
        "T"
    } else {
        " "
    };
    (format!("{idx}{wt}"), idx != " ", wt != " ")
}

fn status_entries(repo: &Repository) -> Result<Vec<FileStatus>, String> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|e| e.message().to_string())?;

    let mut out = Vec::new();
    for entry in statuses.iter() {
        let s = entry.status();
        let (letters, staged, unstaged) = status_letters(s);
        let p = entry.path().unwrap_or("<non-utf8>").to_string();
        let (index, worktree) = (
            letters.chars().next().unwrap_or(' ').to_string(),
            letters.chars().nth(1).unwrap_or(' ').to_string(),
        );
        out.push(FileStatus {
            path: p,
            index,
            worktree,
            staged,
            unstaged,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

#[tauri::command(async)]
pub fn get_status(repository: String) -> Result<Vec<FileStatus>, String> {
    let path = PathBuf::from(&repository);
    let repo = open(&path)?;
    status_entries(&repo)
}

fn stage_path(repo: &Repository, path: &str) -> Result<(), String> {
    let mut index = repo.index().map_err(err)?;
    let full = repo.workdir().unwrap_or_else(|| Path::new(".")).join(path);
    if full.exists() {
        index.add_path(Path::new(path)).map_err(err)?;
    } else {
        index.remove_path(Path::new(path)).map_err(err)?;
    }
    index.write().map_err(err)?;
    Ok(())
}

#[tauri::command(async)]
pub fn stage_file(path: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    stage_path(&repo, &path)
}

#[tauri::command(async)]
pub fn unstage_file(path: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let target = head.as_ref().map(|c| c.as_object());
    repo.reset_default(target, [Path::new(&path)])
        .map_err(|e| e.message().to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn stage_all(repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let mut index = repo.index().map_err(err)?;
    index
        .add_all(["."], IndexAddOption::DEFAULT, None)
        .map_err(err)?;
    index.update_all(["."], None).map_err(err)?;
    index.write().map_err(err)?;
    Ok(())
}

#[tauri::command(async)]
pub fn unstage_all(repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let target = head.as_ref().map(|c| c.as_object());
    repo.reset_default(target, None::<&str>)
        .map_err(|e| e.message().to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn discard_file(path: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;

    let status = repo.status_file(Path::new(&path)).map_err(err)?;
    if status.contains(Status::WT_NEW) {
        let full = repo.workdir().unwrap_or(&p).join(&path);
        if full.is_dir() {
            std::fs::remove_dir_all(&full).map_err(err)?;
        } else {
            std::fs::remove_file(&full).map_err(err)?;
        }
        return Ok(());
    }

    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let target = head.as_ref().map(|c| c.as_object());
    repo.reset_default(target, [Path::new(&path)])
        .map_err(|e| e.message().to_string())?;
    let mut cb = CheckoutBuilder::new();
    cb.path(&path).force();
    repo.checkout_head(Some(&mut cb))
        .map_err(|e| e.message().to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn commit(
    message: String,
    author_name: Option<String>,
    author_email: Option<String>,
    repository: String,
) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".to_string());
    }
    let mut index = repo.index().map_err(err)?;
    let tree_oid = index.write_tree().map_err(|_| {
        "Unable to write tree. Resolve all conflicts before committing.".to_string()
    })?;
    let tree = repo.find_tree(tree_oid).map_err(err)?;

    let sig = match (author_name, author_email) {
        (Some(n), Some(e)) if !n.trim().is_empty() && !e.trim().is_empty() => {
            git2::Signature::now(n.trim(), e.trim()).map_err(err)?
        }
        _ => repo
            .signature()
            .map_err(|_| "Git user.name / user.email are not configured".to_string())?,
    };

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, &message, &tree, &parents)
        .map_err(|e| e.message().to_string())?;
    Ok(oid.to_string())
}

fn log_entries_for(
    repo: &Repository,
    limit: usize,
    skip: usize,
    rev: Option<&str>,
) -> Result<Vec<CommitInfo>, String> {
    let mut walk = repo.revwalk().map_err(err)?;
    match rev.map(str::trim).filter(|r| !r.is_empty()) {
        Some(rev) => {
            let commit = resolve_commit(repo, rev)?;
            walk.push(commit.id()).map_err(err)?;
        }
        None => {
            if walk.push_head().is_err() {
                return Ok(Vec::new());
            }
        }
    }
    walk.set_sorting(git2::Sort::TIME).map_err(err)?;

    let mut out = Vec::new();
    for oid in walk.skip(skip).take(limit) {
        let oid = oid.map_err(err)?;
        let commit = repo.find_commit(oid).map_err(err)?;
        out.push(commit_info(oid, &commit));
    }
    Ok(out)
}

fn log_entries(repo: &Repository, limit: usize, skip: usize) -> Result<Vec<CommitInfo>, String> {
    log_entries_for(repo, limit, skip, None)
}

#[tauri::command(async)]
pub fn get_log(
    limit: Option<usize>,
    skip: Option<usize>,
    rev: Option<String>,
    repository: String,
) -> Result<Vec<CommitInfo>, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    log_entries_for(
        &repo,
        limit.unwrap_or(200),
        skip.unwrap_or(0),
        rev.as_deref(),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchComparison {
    base: String,
    head: String,
    base_oid: String,
    head_oid: String,
    ahead: usize,
    behind: usize,
    commits: Vec<CommitInfo>,
}

#[tauri::command(async)]
pub fn compare_branches(
    base: String,
    head: String,
    limit: Option<usize>,
    repository: String,
) -> Result<BranchComparison, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let base_commit = resolve_commit(&repo, &base)?;
    let head_commit = resolve_commit(&repo, &head)?;
    let (ahead, behind) = repo
        .graph_ahead_behind(head_commit.id(), base_commit.id())
        .unwrap_or((0, 0));

    let mut walk = repo.revwalk().map_err(err)?;
    walk.push(head_commit.id()).map_err(err)?;
    walk.hide(base_commit.id()).map_err(err)?;
    walk.set_sorting(git2::Sort::TIME).map_err(err)?;

    let mut commits = Vec::new();
    for oid in walk.take(limit.unwrap_or(200)) {
        let oid = oid.map_err(err)?;
        let commit = repo.find_commit(oid).map_err(err)?;
        commits.push(commit_info(oid, &commit));
    }

    Ok(BranchComparison {
        base,
        head,
        base_oid: base_commit.id().to_string(),
        head_oid: head_commit.id().to_string(),
        ahead,
        behind,
        commits,
    })
}

fn diff_to_string(repo: &Repository, diff: git2::Diff) -> Result<String, String> {
    let _ = repo;
    let mut text = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        if matches!(origin, '+' | '-' | ' ' | '=') {
            text.push(origin);
        }
        text.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(err)?;
    Ok(text)
}

#[tauri::command(async)]
pub fn get_worktree_diff(
    path: Option<String>,
    staged: bool,
    repository: String,
) -> Result<String, String> {
    let repo_root = PathBuf::from(&repository);
    let repo = open(&repo_root)?;
    let mut opts = DiffOptions::new();
    opts.include_typechange(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    if let Some(path) = &path {
        opts.pathspec(path);
    }

    let diff = if staged {
        let head = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head.as_ref(), None, Some(&mut opts))
    } else {
        repo.diff_index_to_workdir(None, Some(&mut opts))
    }
    .map_err(err)?;

    diff_to_string(&repo, diff)
}

#[tauri::command(async)]
pub fn get_commit_diff(rev: String, repository: String) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let commit = resolve_commit(&repo, &rev)?;
    let tree = commit.tree().map_err(err)?;
    let parent_tree = if commit.parent_count() > 0 {
        commit.parent(0).ok().and_then(|p| p.tree().ok())
    } else {
        None
    };
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(err)?;
    diff_to_string(&repo, diff)
}

fn branch_entries(repo: &Repository) -> Result<Vec<BranchInfo>, String> {
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let head_ancestors = head.as_ref().map(|h| reachable_set(repo, h.id()));
    let mut out = Vec::new();
    for item in repo.branches(None).map_err(err)? {
        let (branch, btype) = item.map_err(err)?;
        let is_remote = btype == BranchType::Remote;
        let name = branch.name().ok().flatten().unwrap_or("").to_string();
        if name.is_empty() || (is_remote && name.ends_with("/HEAD")) {
            continue;
        }
        let merged = if is_remote {
            false
        } else {
            match (&head, &head_ancestors, branch.get().peel_to_commit().ok()) {
                (Some(h), Some(ancestors), Some(tip)) => {
                    h.id() == tip.id() || ancestors.contains(&tip.id())
                }
                _ => false,
            }
        };
        let upstream_branch = branch.upstream().ok();
        let upstream = upstream_branch
            .as_ref()
            .and_then(|u| u.name().ok().flatten().map(String::from));
        let (ahead, behind) = if is_remote {
            (0, 0)
        } else {
            match (
                branch.get().target(),
                upstream_branch.as_ref().and_then(|u| u.get().target()),
            ) {
                (Some(local), Some(remote)) => {
                    repo.graph_ahead_behind(local, remote).unwrap_or((0, 0))
                }
                _ => (0, 0),
            }
        };
        out.push(BranchInfo {
            name,
            is_head: branch.is_head(),
            is_remote,
            merged,
            upstream,
            ahead,
            behind,
            target: branch.get().target().map(|o| o.to_string()),
        });
    }
    out.sort_by(|a, b| (a.is_remote, &a.name).cmp(&(b.is_remote, &b.name)));
    Ok(out)
}

#[tauri::command(async)]
pub fn list_branches(repository: String) -> Result<Vec<BranchInfo>, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    branch_entries(&repo)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchGraphNode {
    name: String,
    is_head: bool,
    is_remote: bool,
    tip: String,
    tip_short: String,
    /// Branch this one was forked from (its "parent"), if determinable.
    parent: Option<String>,
    /// Branches this branch has been merged into.
    merges_into: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchGraph {
    nodes: Vec<BranchGraphNode>,
    head: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    repo: RepoInfo,
    status: Vec<FileStatus>,
    log: Vec<CommitInfo>,
    branches: Vec<BranchInfo>,
    graph: BranchGraph,
    remotes: Vec<RemoteInfo>,
    tags: Vec<TagInfo>,
    stashes: Vec<StashInfo>,
    user: UserConfig,
}

fn reachable_set(repo: &Repository, tip: git2::Oid) -> HashSet<git2::Oid> {
    let mut set = HashSet::new();
    if let Ok(mut walk) = repo.revwalk()
        && walk.push(tip).is_ok()
    {
        for oid in walk.flatten() {
            set.insert(oid);
        }
    }
    set
}

fn branch_graph(repo: &Repository, include_remote: bool) -> Result<BranchGraph, String> {
    struct Br {
        name: String,
        is_head: bool,
        is_remote: bool,
        tip: git2::Oid,
    }

    let mut branches: Vec<Br> = Vec::new();
    for item in repo.branches(None).map_err(err)? {
        let (branch, btype) = item.map_err(err)?;
        let is_remote = btype == BranchType::Remote;
        if is_remote && !include_remote {
            continue;
        }
        let name = match branch.name().ok().flatten() {
            Some(n) => n.to_string(),
            None => continue,
        };
        if is_remote && name.ends_with("/HEAD") {
            continue;
        }
        let tip = match branch.get().target() {
            Some(t) => t,
            None => continue,
        };
        branches.push(Br {
            name,
            is_head: branch.is_head(),
            is_remote,
            tip,
        });
    }

    let (head_name, _, _) = current_branch(repo);

    // First-parent chain and full ancestor set of each branch, used to tell
    // "merged into" apart from a plain linear continuation. Sets are shared
    // between branches that point at the same commit.
    let mut first_parent: Vec<HashSet<git2::Oid>> = Vec::new();
    let mut first_parent_index: HashMap<git2::Oid, usize> = HashMap::new();
    let mut branch_first_parent: Vec<usize> = Vec::with_capacity(branches.len());
    let mut ancestors: Vec<HashSet<git2::Oid>> = Vec::new();
    let mut ancestors_index: HashMap<git2::Oid, usize> = HashMap::new();
    let mut branch_ancestors: Vec<usize> = Vec::with_capacity(branches.len());

    for b in &branches {
        let fp = *first_parent_index.entry(b.tip).or_insert_with(|| {
            let mut set = HashSet::new();
            let mut cur = repo.find_commit(b.tip).ok();
            while let Some(c) = cur {
                if !set.insert(c.id()) {
                    break;
                }
                cur = if c.parent_count() > 0 {
                    c.parent(0).ok()
                } else {
                    None
                };
            }
            first_parent.push(set);
            first_parent.len() - 1
        });
        branch_first_parent.push(fp);

        let anc = *ancestors_index.entry(b.tip).or_insert_with(|| {
            ancestors.push(reachable_set(repo, b.tip));
            ancestors.len() - 1
        });
        branch_ancestors.push(anc);
    }

    let mut nodes: Vec<BranchGraphNode> = Vec::with_capacity(branches.len());
    for (i, b) in branches.iter().enumerate() {
        // Parent: the branch whose line most recently joins B's own first-parent
        // line. Handles linear descendants, forks from a since-advanced branch,
        // and forks from a branch that later merged B back in.
        let mut parent: Option<(String, i64)> = None;
        for (j, a) in branches.iter().enumerate() {
            if i == j || a.tip == b.tip {
                continue;
            }
            let a_is_ancestor = ancestors[branch_ancestors[i]].contains(&a.tip);
            let b_is_ancestor = ancestors[branch_ancestors[j]].contains(&b.tip);

            let shared: Option<git2::Oid> = if a_is_ancestor {
                // A's tip is in B's history: a real ancestor only if it is on
                // B's first-parent line, otherwise A was merged into B.
                if first_parent[branch_first_parent[i]].contains(&a.tip) {
                    Some(a.tip)
                } else {
                    None
                }
            } else if b_is_ancestor {
                // A contains B (e.g. B was merged into A). Find the point on
                // A's first-parent line where B originally forked off.
                let mut cur = repo.find_commit(a.tip).ok();
                let mut found = None;
                while let Some(c) = cur {
                    if c.id() == b.tip || ancestors[branch_ancestors[i]].contains(&c.id()) {
                        found = Some(c.id());
                        break;
                    }
                    cur = if c.parent_count() > 0 {
                        c.parent(0).ok()
                    } else {
                        None
                    };
                }
                match found {
                    Some(sp) if sp != b.tip => Some(sp),
                    _ => None,
                }
            } else {
                // Divergent branches: they forked at their merge-base.
                match repo.merge_base(a.tip, b.tip) {
                    Ok(mb) if first_parent[branch_first_parent[i]].contains(&mb) => Some(mb),
                    _ => None,
                }
            };

            if let Some(sp) = shared {
                let t = repo
                    .find_commit(sp)
                    .map(|c| c.time().seconds())
                    .unwrap_or(0);
                match &parent {
                    Some((_, pt)) if *pt >= t => {}
                    _ => parent = Some((a.name.clone(), t)),
                }
            }
        }
        let mut parent_name = parent.map(|(n, _)| n);

        // Break 2-cycles: if A chose B as its parent too, keep only the link to
        // the branch with the older tip (the likely base branch).
        if let Some(pname) = &parent_name
            && let Some(pj) = branches.iter().position(|x| &x.name == pname)
            && pj != i
        {
            let p_time = repo
                .find_commit(branches[pj].tip)
                .map(|c| c.time().seconds())
                .unwrap_or(0);
            let b_time = repo
                .find_commit(b.tip)
                .map(|c| c.time().seconds())
                .unwrap_or(0);
            let b_is_ancestor_p = ancestors[branch_ancestors[i]].contains(&branches[pj].tip);
            if !b_is_ancestor_p && b_time > p_time {
                // B's tip is newer; assume the other branch is the base,
                // so drop this link.
                parent_name = None;
            }
        }

        // Merges: branch C contains this branch's tip through a merge commit
        // (i.e. reachable, but not on C's first-parent line). Only report direct
        // merges, not ones already folded into C via an earlier merge.
        let mut merges_into = Vec::new();
        for (j, c) in branches.iter().enumerate() {
            if i == j || c.tip == b.tip {
                continue;
            }
            if first_parent[branch_first_parent[j]].contains(&b.tip) {
                continue;
            }
            if !ancestors[branch_ancestors[j]].contains(&b.tip) {
                continue;
            }
            // If B is already in C's first-parent line, this is a transitive merge.
            let c_first_parent = repo.find_commit(c.tip).ok().and_then(|cc| {
                if cc.parent_count() > 0 {
                    cc.parent(0).ok()
                } else {
                    None
                }
            });
            if let Some(fp) = &c_first_parent
                && repo.graph_descendant_of(fp.id(), b.tip).unwrap_or(false)
            {
                continue;
            }
            merges_into.push(c.name.clone());
        }

        nodes.push(BranchGraphNode {
            name: b.name.clone(),
            is_head: b.is_head,
            is_remote: b.is_remote,
            tip: b.tip.to_string(),
            tip_short: short(b.tip),
            parent: parent_name,
            merges_into,
        });
    }

    Ok(BranchGraph {
        nodes,
        head: head_name,
    })
}

#[tauri::command(async)]
pub fn get_branch_graph(repository: String, include_remote: bool) -> Result<BranchGraph, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    branch_graph(&repo, include_remote)
}

#[tauri::command(async)]
pub fn create_branch(name: String, checkout: bool, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let head = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| "Cannot create a branch before the first commit".to_string())?;
    repo.branch(&name, &head, false)
        .map_err(|e| e.message().to_string())?;
    if checkout {
        checkout_branch_impl(&repo, &name)?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn delete_branch(name: String, force: bool, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    if force {
        let refname = format!("refs/heads/{name}");
        let mut reference = repo
            .find_reference(&refname)
            .map_err(|e| e.message().to_string())?;
        reference.delete().map_err(|e| e.message().to_string())?;
    } else {
        let head = repo
            .head()
            .and_then(|h| h.peel_to_commit())
            .map(|c| c.id())
            .map_err(|_| "Cannot delete a branch without a HEAD commit".to_string())?;
        let mut branch = repo
            .find_branch(&name, BranchType::Local)
            .map_err(|e| e.message().to_string())?;
        let tip = branch
            .get()
            .peel_to_commit()
            .map_err(|e| e.message().to_string())?
            .id();
        if tip != head && !repo.graph_descendant_of(head, tip).unwrap_or(false) {
            return Err(format!(
                "Branch \"{name}\" has unmerged commits. Force delete it to discard them."
            ));
        }
        branch.delete().map_err(|e| e.message().to_string())?;
    }
    Ok(())
}

fn checkout_branch_impl(repo: &Repository, name: &str) -> Result<(), String> {
    // Local branch: check it out directly.
    if repo.find_branch(name, BranchType::Local).is_ok() {
        return do_checkout(repo, name);
    }

    // Remote-tracking branch (e.g. "origin/feature"): create a local branch
    // that tracks it, then check that out.
    if repo.find_branch(name, BranchType::Remote).is_ok()
        && let Some((_, short)) = name.split_once('/')
    {
        if repo.find_branch(short, BranchType::Local).is_err() {
            let remote = repo
                .find_branch(name, BranchType::Remote)
                .map_err(|e| e.message().to_string())?;
            let commit = remote
                .get()
                .peel_to_commit()
                .map_err(|e| e.message().to_string())?;
            let mut local = repo
                .branch(short, &commit, false)
                .map_err(|e| e.message().to_string())?;
            local
                .set_upstream(Some(name))
                .map_err(|e| e.message().to_string())?;
        }
        return do_checkout(repo, short);
    }

    do_checkout(repo, name)
}

fn do_checkout(repo: &Repository, name: &str) -> Result<(), String> {
    let (object, reference) = repo
        .revparse_ext(name)
        .map_err(|e| e.message().to_string())?;
    let mut cb = CheckoutBuilder::new();
    cb.safe();
    repo.checkout_tree(&object, Some(&mut cb))
        .map_err(|e| e.message().to_string())?;
    match reference {
        Some(r) => {
            let refname = r.name().ok_or("invalid reference name")?.to_string();
            repo.set_head(&refname)
                .map_err(|e| e.message().to_string())?;
        }
        None => {
            let oid = object.id();
            repo.set_head_detached(oid)
                .map_err(|e| e.message().to_string())?;
        }
    }
    Ok(())
}

#[tauri::command(async)]
pub fn checkout_branch(name: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    checkout_branch_impl(&repo, &name)
}

#[tauri::command(async)]
pub fn merge_branch(name: String, repository: String) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let out = Command::new("git")
        .current_dir(&p)
        .args(["merge", "--no-edit", &name])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if out.status.success() {
        Ok(stdout)
    } else {
        Err(format!("{stdout}{stderr}").trim().to_string())
    }
}

fn remote_entries(repo: &Repository) -> Result<Vec<RemoteInfo>, String> {
    let remotes = repo.remotes().map_err(err)?;
    let mut out = Vec::new();
    for name in remotes.iter().flatten() {
        let url = repo
            .find_remote(name)
            .ok()
            .and_then(|r| r.url().map(String::from))
            .unwrap_or_default();
        out.push(RemoteInfo {
            name: name.to_string(),
            url,
        });
    }
    Ok(out)
}

#[tauri::command(async)]
pub fn list_remotes(repository: String) -> Result<Vec<RemoteInfo>, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    remote_entries(&repo)
}

#[tauri::command(async)]
pub fn add_remote(name: String, url: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    repo.remote(&name, &url)
        .map_err(|e| e.message().to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn remove_remote(name: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    repo.remote_delete(&name)
        .map_err(|e| e.message().to_string())
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if out.status.success() {
        Ok(format!("{stdout}{stderr}").trim().to_string())
    } else {
        Err(format!("{stdout}{stderr}").trim().to_string())
    }
}

/// Runs a git network command while streaming its stderr progress to the UI.
/// Git separates progress updates with `\r`, so lines are split on both `\r`
/// and `\n`; percentage lines are sent as progress but omitted from the result.
fn run_git_streaming(
    dir: &Path,
    args: &[&str],
    app: &AppHandle,
    operation: &str,
) -> Result<String, String> {
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout_thread = child.stdout.take().map(|mut stdout| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf);
            buf
        })
    });

    let mut output = String::new();
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr);
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match reader.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    if byte[0] == b'\r' || byte[0] == b'\n' {
                        if !line.is_empty() {
                            let text = String::from_utf8_lossy(&line).trim().to_string();
                            line.clear();
                            if !text.is_empty() {
                                let _ = app.emit(
                                    "git-progress",
                                    GitProgress {
                                        operation: operation.to_string(),
                                        line: text.clone(),
                                    },
                                );
                                if !text.contains('%') {
                                    output.push_str(&text);
                                    output.push('\n');
                                }
                            }
                        }
                    } else {
                        line.push(byte[0]);
                    }
                }
                Err(_) => break,
            }
        }
        if !line.is_empty() {
            let text = String::from_utf8_lossy(&line).trim().to_string();
            if !text.is_empty() {
                let _ = app.emit(
                    "git-progress",
                    GitProgress {
                        operation: operation.to_string(),
                        line: text.clone(),
                    },
                );
                if !text.contains('%') {
                    output.push_str(&text);
                    output.push('\n');
                }
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for git: {e}"))?;
    let stdout = stdout_thread
        .and_then(|h| h.join().ok())
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_default();

    if status.success() {
        Ok(format!("{stdout}{output}").trim().to_string())
    } else {
        Err(format!("{stdout}{output}").trim().to_string())
    }
}

#[tauri::command(async)]
pub fn fetch(app: AppHandle, remote: Option<String>, repository: String) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let remote = remote.unwrap_or_else(|| "--all".to_string());
    run_git_streaming(&p, &["fetch", &remote, "--prune"], &app, "fetch")
}

#[tauri::command(async)]
pub fn pull(
    app: AppHandle,
    remote: Option<String>,
    branch: Option<String>,
    repository: String,
) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let mut args: Vec<String> = vec!["pull".to_string(), "--prune".to_string()];
    if let Some(r) = remote
        && !r.is_empty()
    {
        args.push(r);
        if let Some(b) = branch
            && !b.is_empty()
        {
            args.push(b);
        }
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_git_streaming(&p, &refs, &app, "pull")
}

#[tauri::command(async)]
pub fn push(
    app: AppHandle,
    remote: Option<String>,
    branch: Option<String>,
    set_upstream: bool,
    repository: String,
) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let mut args: Vec<String> = vec!["push".to_string()];
    if set_upstream {
        args.push("-u".to_string());
    }
    let remote = remote.unwrap_or_else(|| "origin".to_string());
    args.push(remote);
    if let Some(b) = branch
        && !b.is_empty()
    {
        args.push(b);
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_git_streaming(&p, &refs, &app, "push")
}

fn tag_entries(repo: &Repository) -> Result<Vec<TagInfo>, String> {
    let mut out = Vec::new();
    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name).to_string();
        if let Some(tag_name) = name.strip_prefix("refs/tags/") {
            if tag_name.ends_with("^{}") {
                return true;
            }
            let (message, annotated) = match repo.find_tag(oid) {
                Ok(tag) => (tag.message().unwrap_or("").trim().to_string(), true),
                Err(_) => (String::new(), false),
            };
            let target = repo
                .find_object(oid, None)
                .ok()
                .and_then(|o| o.peel_to_commit().ok())
                .map(|c| c.id().to_string())
                .unwrap_or_else(|| oid.to_string());
            out.push(TagInfo {
                name: tag_name.to_string(),
                target,
                message,
                annotated,
            });
        }
        true
    })
    .map_err(err)?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[tauri::command(async)]
pub fn list_tags(repository: String) -> Result<Vec<TagInfo>, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    tag_entries(&repo)
}

#[tauri::command(async)]
pub fn create_tag(
    name: String,
    message: Option<String>,
    target: Option<String>,
    repository: String,
) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    let obj = match target {
        Some(t) if !t.trim().is_empty() => {
            let t = t.trim();
            match git2::Oid::from_str(t) {
                Ok(oid) => repo.find_object(oid, None).map_err(err)?,
                Err(_) => repo
                    .revparse_single(t)
                    .map_err(|e| e.message().to_string())?,
            }
        }
        _ => repo
            .head()
            .and_then(|h| h.peel_to_commit())
            .map(|c| c.into_object())
            .map_err(|_| "No commit to tag".to_string())?,
    };
    match message {
        Some(m) if !m.trim().is_empty() => {
            let sig = repo.signature().map_err(err)?;
            repo.tag(&name, &obj, &sig, &m, false)
                .map_err(|e| e.message().to_string())?;
        }
        _ => {
            repo.tag_lightweight(&name, &obj, false)
                .map_err(|e| e.message().to_string())?;
        }
    }
    Ok(())
}

#[tauri::command(async)]
pub fn delete_tag(name: String, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    repo.tag_delete(&name).map_err(|e| e.message().to_string())
}

fn stash_entries(repo: &mut Repository) -> Result<Vec<StashInfo>, String> {
    let mut out = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        out.push(StashInfo {
            index,
            message: message.to_string(),
            oid: oid.to_string(),
        });
        true
    })
    .map_err(err)?;
    Ok(out)
}

#[tauri::command(async)]
pub fn list_stashes(repository: String) -> Result<Vec<StashInfo>, String> {
    let p = PathBuf::from(&repository);
    let mut repo = open(&p)?;
    stash_entries(&mut repo)
}

#[tauri::command(async)]
pub fn create_stash(message: Option<String>, repository: String) -> Result<String, String> {
    let p = PathBuf::from(&repository);
    let mut repo = open(&p)?;
    let sig = repo
        .signature()
        .map_err(|_| "Git user.name / user.email are not configured".to_string())?;
    let msg = message.unwrap_or_default();
    let oid = repo
        .stash_save(&sig, &msg, Some(git2::StashFlags::INCLUDE_UNTRACKED))
        .map_err(|e| e.message().to_string())?;
    Ok(oid.to_string())
}

#[tauri::command(async)]
pub fn apply_stash(index: usize, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let mut repo = open(&p)?;
    repo.stash_apply(index, None)
        .map_err(|e| e.message().to_string())
}

#[tauri::command(async)]
pub fn pop_stash(index: usize, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let mut repo = open(&p)?;
    repo.stash_pop(index, None)
        .map_err(|e| e.message().to_string())
}

#[tauri::command(async)]
pub fn drop_stash(index: usize, repository: String) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let mut repo = open(&p)?;
    repo.stash_drop(index).map_err(|e| e.message().to_string())
}

fn user_config(repo: &Repository) -> Result<UserConfig, String> {
    let cfg = repo.config().map_err(err)?;
    Ok(UserConfig {
        name: cfg.get_string("user.name").unwrap_or_default(),
        email: cfg.get_string("user.email").unwrap_or_default(),
    })
}

#[tauri::command(async)]
pub fn get_user_config(repository: String) -> Result<UserConfig, String> {
    let p = PathBuf::from(&repository);
    let repo = open(&p)?;
    user_config(&repo)
}

#[tauri::command(async)]
pub fn set_user_config(
    name: String,
    email: String,
    global: bool,
    repository: String,
) -> Result<(), String> {
    let p = PathBuf::from(&repository);
    let scope = if global { "--global" } else { "--local" };
    if !name.trim().is_empty() {
        run_git(&p, &["config", scope, "user.name", name.trim()])?;
    }
    if !email.trim().is_empty() {
        run_git(&p, &["config", scope, "user.email", email.trim()])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestRepo {
        path: PathBuf,
        repo: Repository,
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn init() -> TestRepo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("gitweft-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@example.com").unwrap();
        }
        TestRepo { path, repo }
    }

    fn commit_file(repo: &Repository, name: &str, content: &str) -> git2::Oid {
        let path = repo.workdir().unwrap().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents)
            .unwrap()
    }

    fn default_branch(repo: &Repository) -> String {
        repo.head().unwrap().shorthand().unwrap().to_string()
    }

    #[test]
    fn stages_deleted_file() {
        let t = init();
        commit_file(&t.repo, "a.txt", "hello");
        std::fs::remove_file(t.path.join("a.txt")).unwrap();
        stage_path(&t.repo, "a.txt").unwrap();
        let index = t.repo.index().unwrap();
        assert!(index.get_path(Path::new("a.txt"), 0).is_none());
    }

    #[test]
    fn deletes_merged_branch_without_force() {
        let t = init();
        commit_file(&t.repo, "a.txt", "one");
        let head = t.repo.head().unwrap().peel_to_commit().unwrap();
        t.repo.branch("merged", &head, false).unwrap();
        let path = t.path.to_string_lossy().to_string();
        assert!(delete_branch("merged".into(), false, path).is_ok());
    }

    #[test]
    fn refuses_to_delete_unmerged_branch_without_force() {
        let t = init();
        commit_file(&t.repo, "a.txt", "one");
        let main = default_branch(&t.repo);
        let head = t.repo.head().unwrap().peel_to_commit().unwrap();
        t.repo.branch("feature", &head, false).unwrap();
        t.repo.set_head("refs/heads/feature").unwrap();
        commit_file(&t.repo, "b.txt", "two");
        t.repo.set_head(&format!("refs/heads/{main}")).unwrap();
        let path = t.path.to_string_lossy().to_string();
        assert!(delete_branch("feature".into(), false, path.clone()).is_err());
        assert!(delete_branch("feature".into(), true, path).is_ok());
    }

    #[test]
    fn filters_remote_head_and_marks_merged_branches() {
        let t = init();
        commit_file(&t.repo, "a.txt", "one");
        let head = t.repo.head().unwrap().peel_to_commit().unwrap();
        let main = default_branch(&t.repo);
        t.repo.branch("merged", &head, false).unwrap();
        t.repo
            .reference(
                &format!("refs/remotes/origin/{main}"),
                head.id(),
                true,
                "test",
            )
            .unwrap();
        t.repo
            .reference_symbolic(
                "refs/remotes/origin/HEAD",
                &format!("refs/remotes/origin/{main}"),
                true,
                "test",
            )
            .unwrap();

        let branches = branch_entries(&t.repo).unwrap();
        assert!(branches.iter().all(|b| b.name != "origin/HEAD"));
        assert!(branches.iter().find(|b| b.name == "merged").unwrap().merged);
    }

    #[test]
    fn paginates_log() {
        let t = init();
        commit_file(&t.repo, "a.txt", "0");
        for i in 1..5 {
            commit_file(&t.repo, "a.txt", &i.to_string());
        }
        let all = log_entries(&t.repo, 10, 0).unwrap();
        let page = log_entries(&t.repo, 2, 1).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].oid, all[1].oid);
        assert_eq!(page[1].oid, all[2].oid);
    }

    #[test]
    fn resolves_branch_names_and_compares_branches() {
        let t = init();
        commit_file(&t.repo, "a.txt", "0");
        let main = default_branch(&t.repo);
        let head = t.repo.head().unwrap().peel_to_commit().unwrap();
        t.repo.branch("feature", &head, false).unwrap();
        t.repo.set_head("refs/heads/feature").unwrap();
        commit_file(&t.repo, "b.txt", "1");
        t.repo.set_head(&format!("refs/heads/{main}")).unwrap();
        let feature_tip = t
            .repo
            .find_reference("refs/heads/feature")
            .unwrap()
            .target()
            .unwrap()
            .to_string();
        let path = t.path.to_string_lossy().to_string();

        let log = log_entries_for(&t.repo, 10, 0, Some("feature")).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].oid, feature_tip);

        let comparison =
            compare_branches(main.clone(), "feature".into(), Some(10), path.clone()).unwrap();
        assert_eq!(comparison.ahead, 1);
        assert_eq!(comparison.behind, 0);
        assert_eq!(comparison.head_oid, feature_tip);
        assert_eq!(comparison.commits.len(), 1);
        assert_eq!(comparison.commits[0].oid, feature_tip);

        let diff = get_commit_diff("feature".into(), path.clone()).unwrap();
        assert!(diff.contains("b.txt"));
        assert!(get_commit_diff("does-not-exist".into(), path).is_err());
    }

    #[test]
    fn discards_worktree_changes_without_peel_error() {
        let t = init();
        commit_file(&t.repo, "a.txt", "hello");
        std::fs::write(t.path.join("a.txt"), "changed").unwrap();
        let path = t.path.to_string_lossy().to_string();
        discard_file("a.txt".into(), path).unwrap();
        assert_eq!(
            std::fs::read_to_string(t.path.join("a.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn unstages_file_without_peel_error() {
        let t = init();
        commit_file(&t.repo, "a.txt", "hello");
        std::fs::write(t.path.join("a.txt"), "changed").unwrap();
        stage_path(&t.repo, "a.txt").unwrap();
        let path = t.path.to_string_lossy().to_string();
        unstage_file("a.txt".into(), path).unwrap();
        let status = t.repo.status_file(Path::new("a.txt")).unwrap();
        assert!(!status.contains(Status::INDEX_MODIFIED));
        assert!(status.contains(Status::WT_MODIFIED));
    }

    #[test]
    fn snapshot_gathers_repository_state() {
        let t = init();
        commit_file(&t.repo, "a.txt", "one");
        commit_file(&t.repo, "b.txt", "two");
        std::fs::write(t.path.join("c.txt"), "new").unwrap();
        let main = default_branch(&t.repo);
        let snap = get_snapshot(t.path.to_string_lossy().to_string(), Some(10), false).unwrap();
        assert_eq!(snap.repo.branch, main);
        assert!(snap.repo.dirty);
        assert_eq!(snap.status.len(), 1);
        assert_eq!(snap.log.len(), 2);
        assert!(snap.branches.iter().any(|b| b.is_head));
        assert_eq!(snap.user.name, "Test");
        assert!(snap.graph.nodes.iter().any(|n| n.is_head));
    }
}
