#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_step_steam_library(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.step.hidden#step-steam-library"), |dom| {
            dom.el(el("div.card"), |dom| {
                dom.el(el("h2.shortcut-header"), |dom| {
                    dom.text(self.t("steamLibrary.title"));
                });
                dom.el(el("p.shortcut-description"), |dom| {
                    dom.text(self.t("steamLibrary.description"));
                });
                dom.el(self.ui.el("div.emu-release-status#steam-library-status"), |dom| {
                    self.label("steam-library-status", "steamLibrary.detecting", dom);
                });
                dom.el(el("div.shortcut-exe-section"), |dom| {
                    if dom.button(el("label.settings-section__label[for=steam-exe-path]"), |dom| {
                        dom.text(self.t("steamLibrary.exeLabel"));
                    }).clicked() {
                        self.label_click("steam-exe-path");
                    }
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#steam-exe-path[readonly][type=text]");
                            let placeholder = self.ui.attr("steam-exe-path", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("shortcut.exePlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("steam-exe-path"), &placeholder, false);
                            self.input_events("steam-exe-path", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.dd-path__browse-btn#btn-steam-browse-exe"), |dom| {
                            self.label("btn-steam-browse-exe", "shortcut.browse", dom);
                        }).clicked() {
                            self.click("btn-steam-browse-exe");
                        }
                    });
                    dom.el(el("p.dd-path__hint"), |dom| {
                        dom.text(self.t("steamLibrary.exeHint"));
                    });
                });
                dom.el(self.ui.el("div.shortcut-detected-section.hidden#steam-detected-section"), |dom| {
                    if dom.button(self.ui.el("button.settings-advanced__toggle#btn-steam-toggle-detected"), |dom| {
                        dom.el(el("span.settings-advanced__arrow"), |dom| {
                            self.fill("steam-detected-arrow", dom);
                        });
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("shortcut.otherDetected"));
                        });
                    }).clicked() {
                        self.click("btn-steam-toggle-detected");
                    }
                    dom.el(self.ui.el("div.shortcut-detected-list.hidden#steam-detected-list"), |dom| {
                        self.fill("steam-detected-list", dom);
                    });
                });
                dom.el(el("div.settings-field"), |dom| {
                    if dom.button(el("label.settings-field__label[for=steam-game-name]"), |dom| {
                        dom.text(self.t("steamLibrary.gameNameLabel"));
                    }).clicked() {
                        self.label_click("steam-game-name");
                    }
                    {
                        let element = self.ui.el("input.dd-path__input#steam-game-name[type=text]");
                        let placeholder = self.ui.attr("steam-game-name", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("steamLibrary.gameNamePlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("steam-game-name"), &placeholder, false);
                        self.input_events("steam-game-name", &response);
                    }
                });
                dom.el(el("div.settings-field"), |dom| {
                    if dom.button(el("label.settings-field__label[for=steam-launch-options]"), |dom| {
                        dom.text(self.t("steamLibrary.launchOptionsLabel"));
                    }).clicked() {
                        self.label_click("steam-launch-options");
                    }
                    {
                        let element = self.ui.el("input.dd-path__input#steam-launch-options[type=text]");
                        let placeholder = self.ui.attr("steam-launch-options", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("steamLibrary.launchOptionsPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("steam-launch-options"), &placeholder, false);
                        self.input_events("steam-launch-options", &response);
                    }
                    dom.el(el("p.dd-path__hint"), |dom| {
                        dom.text(self.t("steamLibrary.launchOptionsHint"));
                    });
                });
                dom.el(self.ui.el("div.completion-message.hidden#steam-library-result"), |dom| {
                    self.fill("steam-library-result", dom);
                });
                dom.el(el("div.card__footer"), |dom| {
                    dom.el(el("div.card__footer-left"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-steam-skip"), |dom| {
                            self.label("btn-steam-skip", "steamLibrary.skip", dom);
                        }).clicked() {
                            self.click("btn-steam-skip");
                        }
                    });
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-steam-add"), |dom| {
                        self.label("btn-steam-add", "steamLibrary.add", dom);
                    }).clicked() {
                        self.click("btn-steam-add");
                    }
                });
            });
        });
    }
}
