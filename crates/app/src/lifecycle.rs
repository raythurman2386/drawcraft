//! Document lifecycle prompts: crash recovery and external-change detection.

use drawcraft_command::ids;
use drawcraft_session::RecoveryCandidate;
use egui::{Align2, RichText};
use serde_json::json;

use crate::DrawCraftApp;
use crate::panels::action_button;
use crate::theme;

/// Data shown by the recovery modal.
#[derive(Clone, Debug)]
pub struct RecoveryPrompt {
    pub candidates: Vec<RecoveryCandidate>,
    pub index: usize,
}

/// Data shown by the external-change modal.
#[derive(Clone, Debug)]
pub struct ExternalChangePrompt {
    pub path: String,
}

impl DrawCraftApp {
    /// Scan for recovery candidates after launch / open.
    pub(crate) fn check_recovery(&mut self) {
        let candidates = self.session.list_recovery_candidates();
        if candidates.is_empty() {
            return;
        }
        self.state.recovery_prompt = Some(RecoveryPrompt {
            candidates,
            index: 0,
        });
    }

    pub(crate) fn show_recovery_prompt(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.state.recovery_prompt.clone() else {
            return;
        };
        let Some(candidate) = prompt.candidates.get(prompt.index) else {
            self.state.recovery_prompt = None;
            return;
        };
        let p = theme::palette(self.session.editor.view.dark);
        let name = candidate
            .source_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let mut open = true;
        let mut restore = false;
        let mut discard = false;
        egui::Window::new("Recover unsaved work")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("DrawCraft found an autosave newer than the file on disk.")
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{name} · {} shapes · autosave revision {}",
                        candidate.shapes, candidate.meta.revision
                    ))
                    .size(12.0)
                    .color(p.secondary),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if action_button(ui, p, "Restore").clicked() {
                        restore = true;
                    }
                    if ui.button("Discard").clicked() {
                        discard = true;
                    }
                });
            });
        if !open {
            self.state.recovery_prompt = None;
            return;
        }
        if restore {
            let autosave = candidate.autosave_path.display().to_string();
            self.run(
                ids::SESSION_RECOVER,
                json!({"action": "restore", "autosave": autosave}),
            );
            self.state.recovery_prompt = None;
            self.state.set_status("Restored from autosave");
        } else if discard {
            let autosave = candidate.autosave_path.display().to_string();
            self.run(
                ids::SESSION_RECOVER,
                json!({"action": "discard", "autosave": autosave}),
            );
            self.state.recovery_prompt = None;
            self.state.set_status("Discarded autosave");
        }
    }

    pub(crate) fn show_external_change_prompt(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.state.external_change_prompt.clone() else {
            return;
        };
        let p = theme::palette(self.session.editor.view.dark);
        let mut open = true;
        let mut reload = false;
        let mut keep = false;
        egui::Window::new("File changed on disk")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("This document was modified outside DrawCraft.")
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(prompt.path.clone())
                        .size(12.0)
                        .color(p.secondary),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if action_button(ui, p, "Reload").clicked() {
                        reload = true;
                    }
                    if ui.button("Keep mine").clicked() {
                        keep = true;
                    }
                });
            });
        if !open {
            self.state.external_change_prompt = None;
            return;
        }
        if reload {
            self.run(ids::SESSION_RELOAD, json!({"force": true}));
            self.state.external_change_prompt = None;
            self.state.set_status("Reloaded from disk");
        } else if keep {
            self.run(ids::SESSION_KEEP_MINE, json!({}));
            self.state.external_change_prompt = None;
            self.state
                .set_status("Keeping your edits; next save will overwrite disk");
        }
    }

    pub(crate) fn maybe_prompt_external(&mut self, changed: bool) {
        if !changed || self.state.recovery_prompt.is_some() {
            return;
        }
        if self.state.external_change_prompt.is_some() {
            return;
        }
        let path = self
            .session
            .current_path()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        self.state.external_change_prompt = Some(ExternalChangePrompt { path });
    }
}
