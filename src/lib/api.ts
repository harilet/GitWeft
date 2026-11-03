import { invoke } from "@tauri-apps/api/core";

export interface RepoInfo {
  path: string;
  name: string;
  branch: string;
  head: string;
  detached: boolean;
  unborn: boolean;
  dirty: boolean;
  ahead: number;
  behind: number;
}

export interface FileStatus {
  path: string;
  index: string;
  worktree: string;
  staged: boolean;
  unstaged: boolean;
}

export interface CommitInfo {
  oid: string;
  short: string;
  summary: string;
  message: string;
  author: string;
  email: string;
  time: number;
  parents: string[];
}

export interface BranchInfo {
  name: string;
  isHead: boolean;
  isRemote: boolean;
  merged: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
  target: string | null;
}

export interface RemoteInfo {
  name: string;
  url: string;
}

export interface TagInfo {
  name: string;
  target: string;
  message: string;
  annotated: boolean;
}

export interface StashInfo {
  index: number;
  message: string;
  oid: string;
}

export interface UserConfig {
  name: string;
  email: string;
}

export interface BranchGraphNode {
  name: string;
  isHead: boolean;
  isRemote: boolean;
  tip: string;
  tipShort: string;
  parent: string | null;
  mergesInto: string[];
}

export interface BranchGraph {
  nodes: BranchGraphNode[];
  head: string;
}

export interface BranchComparison {
  base: string;
  head: string;
  baseOid: string;
  headOid: string;
  ahead: number;
  behind: number;
  commits: CommitInfo[];
}

export interface Snapshot {
  repo: RepoInfo;
  status: FileStatus[];
  log: CommitInfo[];
  branches: BranchInfo[];
  graph: BranchGraph;
  remotes: RemoteInfo[];
  tags: TagInfo[];
  stashes: StashInfo[];
  user: UserConfig;
}

export const git = {
  openRepository: (path: string) => invoke<RepoInfo>("open_repository", { path }),
  initRepository: (path: string) => invoke<RepoInfo>("init_repository", { path }),
  cloneRepository: (url: string, destination: string) =>
    invoke<RepoInfo>("clone_repository", { url, destination }),
  getRepositoryInfo: (repository: string) =>
    invoke<RepoInfo>("get_repository_info", { repository }),
  getSnapshot: (repository: string, logLimit = 300, includeRemote = false) =>
    invoke<Snapshot>("get_snapshot", { repository, logLimit, includeRemote }),

  getStatus: (repository: string) => invoke<FileStatus[]>("get_status", { repository }),
  stageFile: (repository: string, path: string) =>
    invoke<void>("stage_file", { repository, path }),
  unstageFile: (repository: string, path: string) =>
    invoke<void>("unstage_file", { repository, path }),
  stageAll: (repository: string) => invoke<void>("stage_all", { repository }),
  unstageAll: (repository: string) => invoke<void>("unstage_all", { repository }),
  discardFile: (repository: string, path: string) =>
    invoke<void>("discard_file", { repository, path }),
  commit: (
    repository: string,
    message: string,
    authorName?: string,
    authorEmail?: string,
  ) => invoke<string>("commit", { repository, message, authorName, authorEmail }),

  getLog: (repository: string, limit = 300, skip = 0, rev?: string | null) =>
    invoke<CommitInfo[]>("get_log", {
      repository,
      limit,
      skip,
      rev: rev ?? null,
    }),
  getWorktreeDiff: (repository: string, path: string | null, staged: boolean) =>
    invoke<string>("get_worktree_diff", { repository, path, staged }),
  getCommitDiff: (repository: string, rev: string) =>
    invoke<string>("get_commit_diff", { repository, rev }),
  compareBranches: (
    repository: string,
    base: string,
    head: string,
    limit = 200,
  ) =>
    invoke<BranchComparison>("compare_branches", {
      repository,
      base,
      head,
      limit,
    }),

  listBranches: (repository: string) =>
    invoke<BranchInfo[]>("list_branches", { repository }),
  getBranchGraph: (repository: string, includeRemote: boolean) =>
    invoke<BranchGraph>("get_branch_graph", { repository, includeRemote }),
  createBranch: (repository: string, name: string, checkout: boolean) =>
    invoke<void>("create_branch", { repository, name, checkout }),
  deleteBranch: (repository: string, name: string, force: boolean) =>
    invoke<void>("delete_branch", { repository, name, force }),
  checkoutBranch: (repository: string, name: string) =>
    invoke<void>("checkout_branch", { repository, name }),
  mergeBranch: (repository: string, name: string) =>
    invoke<string>("merge_branch", { repository, name }),

  listRemotes: (repository: string) => invoke<RemoteInfo[]>("list_remotes", { repository }),
  addRemote: (repository: string, name: string, url: string) =>
    invoke<void>("add_remote", { repository, name, url }),
  removeRemote: (repository: string, name: string) =>
    invoke<void>("remove_remote", { repository, name }),
  fetch: (repository: string, remote: string | null) =>
    invoke<string>("fetch", { repository, remote }),
  pull: (repository: string, remote: string | null, branch: string | null) =>
    invoke<string>("pull", { repository, remote, branch }),
  push: (
    repository: string,
    remote: string | null,
    branch: string | null,
    setUpstream: boolean,
  ) => invoke<string>("push", { repository, remote, branch, setUpstream }),

  listTags: (repository: string) => invoke<TagInfo[]>("list_tags", { repository }),
  createTag: (
    repository: string,
    name: string,
    message: string | null,
    target: string | null,
  ) => invoke<void>("create_tag", { repository, name, message, target }),
  deleteTag: (repository: string, name: string) =>
    invoke<void>("delete_tag", { repository, name }),

  listStashes: (repository: string) => invoke<StashInfo[]>("list_stashes", { repository }),
  createStash: (repository: string, message: string | null) =>
    invoke<string>("create_stash", { repository, message }),
  applyStash: (repository: string, index: number) =>
    invoke<void>("apply_stash", { repository, index }),
  popStash: (repository: string, index: number) =>
    invoke<void>("pop_stash", { repository, index }),
  dropStash: (repository: string, index: number) =>
    invoke<void>("drop_stash", { repository, index }),

  getUserConfig: (repository: string) =>
    invoke<UserConfig>("get_user_config", { repository }),
  setUserConfig: (repository: string, name: string, email: string, global: boolean) =>
    invoke<void>("set_user_config", { repository, name, email, global }),
};
