mod ai;
mod git;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            git::open_repository,
            git::init_repository,
            git::get_repository_info,
            git::get_snapshot,
            git::clone_repository,
            git::get_status,
            git::stage_file,
            git::unstage_file,
            git::stage_all,
            git::unstage_all,
            git::discard_file,
            git::commit,
            git::get_log,
            git::get_worktree_diff,
            git::get_commit_diff,
            git::compare_branches,
            git::list_branches,
            git::get_branch_graph,
            git::create_branch,
            git::delete_branch,
            git::checkout_branch,
            git::merge_branch,
            git::list_remotes,
            git::add_remote,
            git::remove_remote,
            git::fetch,
            git::pull,
            git::push,
            git::list_tags,
            git::create_tag,
            git::delete_tag,
            git::list_stashes,
            git::create_stash,
            git::apply_stash,
            git::pop_stash,
            git::drop_stash,
            git::get_user_config,
            git::set_user_config,
            ai::ai_chat,
            ai::ai_list_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
