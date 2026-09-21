use super::*;

pub(super) fn panel(
    this: &mut PopoverHost,
    repo_id: RepoId,
    area: DiffArea,
    path: Option<std::path::PathBuf>,
    cx: &mut gpui::Context<PopoverHost>,
) -> gpui::Div {
    let theme = this.theme;
    let selected_paths_count = {
        let pane = this.details_pane.read(cx);
        pane.status_multi_selection
            .get(&repo_id)
            .map(|sel| sel.selected_count_for_area(area))
            .unwrap_or(0)
    };

    let (_count, detail, can_discard) = match path.as_ref() {
        Some(clicked_path) => {
            let (_use_selection, selected_count) = {
                let pane = this.details_pane.read(cx);
                let selection = pane
                    .status_multi_selection
                    .get(&repo_id)
                    .map(|sel| sel.selected_paths_for_area(area))
                    .unwrap_or(&[]);

                let use_selection =
                    selection.len() > 1 && selection.iter().any(|p| p == clicked_path);
                let selected_count = if use_selection { selection.len() } else { 1 };
                (use_selection, selected_count)
            };

            let detail = if selected_count == 1 {
                clicked_path.display().to_string()
            } else {
                format!("{selected_count} files")
            };
            (selected_count, detail, true)
        }
        None => {
            if selected_paths_count == 0 {
                (0, "No files selected.".to_string(), false)
            } else if selected_paths_count == 1 {
                let selected_path = this
                    .details_pane
                    .read(cx)
                    .status_multi_selection
                    .get(&repo_id)
                    .and_then(|sel| sel.first_selected_for_area(area))
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "file".to_string());
                (1, selected_path, true)
            } else {
                (
                    selected_paths_count,
                    format!("{selected_paths_count} files"),
                    true,
                )
            }
        }
    };

    // Discarding a submodule row runs `git submodule update --checkout --force`,
    // which resets the nested repository: say so, because "working tree changes"
    // reads as "changes in this repository" and the loss is inside another one.
    let submodule_count = {
        let paths: Vec<std::path::PathBuf> = match path.as_ref() {
            Some(clicked_path) => {
                let pane = this.details_pane.read(cx);
                let selection = pane
                    .status_multi_selection
                    .get(&repo_id)
                    .map(|sel| sel.selected_paths_for_area(area))
                    .unwrap_or(&[]);
                if selection.len() > 1 && selection.iter().any(|p| p == clicked_path) {
                    selection.to_vec()
                } else {
                    vec![clicked_path.clone()]
                }
            }
            None => {
                let pane = this.details_pane.read(cx);
                pane.status_multi_selection
                    .get(&repo_id)
                    .map(|sel| sel.selected_paths_for_area(area).to_vec())
                    .unwrap_or_default()
            }
        };
        this.state
            .repos
            .iter()
            .find(|repo| repo.id == repo_id)
            .map(|repo| {
                paths
                    .iter()
                    .filter(|p| repo.working_tree_path_is_submodule(area, p))
                    .count()
            })
            .unwrap_or(0)
    };

    let dialog = ConfirmDialog::new("Discard changes", DIALOG_420_WIDTH).text(
        theme,
        format!("This will discard working tree changes for {detail}."),
    );
    let dialog = match submodule_count {
        0 => dialog,
        1 => dialog.note(
            theme,
            "One of these is a submodule: it will be reset to the commit this \
             repository records, discarding uncommitted work inside it.",
        ),
        n => dialog.note(
            theme,
            format!(
                "{n} of these are submodules: they will be reset to the commits \
                 this repository records, discarding uncommitted work inside them."
            ),
        ),
    };
    dialog.render(
        theme,
        dialog_cancel_button(
            "discard_changes_cancel",
            "discard_changes_cancel_hint",
            theme,
            cx,
        ),
        components::Button::new("discard_changes_go", "Discard")
            .style(components::ButtonStyle::Danger)
            .disabled(!can_discard)
            .on_click(theme, cx, move |this, _e, _w, cx| {
                this.discard_worktree_changes_confirmed(repo_id, area, path.clone(), cx);
                this.close_popover(cx);
            }),
        cx,
    )
}
