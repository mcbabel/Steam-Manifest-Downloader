#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_step_shortcut(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.step.hidden#step-shortcut"), |dom| {
            dom.el(el("div.card"), |dom| {
                dom.el(el("h2.shortcut-header"), |dom| {
                    dom.text(self.t("shortcut.title"));
                });
                dom.el(el("p.shortcut-description"), |dom| {
                    dom.text(self.t("shortcut.description"));
                });
                dom.el(el("div.shortcut-exe-section"), |dom| {
                    if dom.button(el("label.settings-section__label[for=shortcut-exe-path]"), |dom| {
                        dom.text(self.t("shortcut.exeLabel"));
                    }).clicked() {
                        self.label_click("shortcut-exe-path");
                    }
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#shortcut-exe-path[readonly][type=text]");
                            let placeholder = self.ui.attr("shortcut-exe-path", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("shortcut.exePlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("shortcut-exe-path"), &placeholder, false);
                            self.input_events("shortcut-exe-path", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.dd-path__browse-btn#btn-browse-exe").title_or(self.t("shortcut.browse")), |dom| {
                            self.label("btn-browse-exe", "shortcut.browse", dom);
                        }).clicked() {
                            self.click("btn-browse-exe");
                        }
                    });
                    dom.el(el("p.dd-path__hint"), |dom| {
                        dom.text(self.t("shortcut.exeHint"));
                    });
                });
                dom.el(self.ui.el("div.shortcut-detected-section.hidden#shortcut-detected-section"), |dom| {
                    if dom.button(self.ui.el("button.settings-advanced__toggle#btn-toggle-detected"), |dom| {
                        dom.el(el("span.settings-advanced__arrow"), |dom| {
                            self.fill("shortcut-detected-arrow", dom);
                        });
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("shortcut.otherDetected"));
                        });
                    }).clicked() {
                        self.click("btn-toggle-detected");
                    }
                    dom.el(self.ui.el("div.shortcut-detected-list.hidden#shortcut-detected-list"), |dom| {
                        self.fill("shortcut-detected-list", dom);
                    });
                });
                dom.el(el("div.shortcut-options"), |dom| {
                    dom.el(el("div.shortcut-option"), |dom| {
                        dom.el(el("span.shortcut-option__label"), |dom| {
                            dom.text(self.t("shortcut.desktopShortcut"));
                        });
                        if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("shortcut-desktop")), |dom| {
                            dom.empty(self.ui.el("input#shortcut-desktop[type=checkbox]").state(CHECKED, self.ui.checked("shortcut-desktop")));
                            dom.empty(el("span.toggle-switch__slider"));
                        }).clicked() {
                            self.toggle("shortcut-desktop");
                        }
                    });
                    dom.el(el("div.shortcut-option"), |dom| {
                        dom.el(el("span.shortcut-option__label"), |dom| {
                            dom.text(self.t("shortcut.startMenuShortcut"));
                        });
                        if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("shortcut-startmenu")), |dom| {
                            dom.empty(self.ui.el("input#shortcut-startmenu[type=checkbox]").state(CHECKED, self.ui.checked("shortcut-startmenu")));
                            dom.empty(el("span.toggle-switch__slider"));
                        }).clicked() {
                            self.toggle("shortcut-startmenu");
                        }
                    });
                    dom.el(self.ui.el("div.shortcut-option.shortcut-option--full#shortcut-steam-row"), |dom| {
                        dom.el(el("div.shortcut-option__text"), |dom| {
                            dom.el(el("span.shortcut-option__label"), |dom| {
                                dom.text(self.t("steamLibrary.windowsToggle"));
                            });
                            dom.el(el("p.shortcut-option__hint"), |dom| {
                                self.label("shortcut-steam-hint", "steamLibrary.windowsToggleHint", dom);
                            });
                            if dom.button(self.ui.el("button.btn.btn--small.btn--outline.shortcut-option__action.hidden#btn-shortcut-steam-choose"), |dom| {
                                self.label("btn-shortcut-steam-choose", "steamLibrary.chooseFolder", dom);
                            }).clicked() {
                                self.click("btn-shortcut-steam-choose");
                            }
                        });
                        if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("shortcut-steam-library")), |dom| {
                            dom.empty(self.ui.el("input#shortcut-steam-library[type=checkbox]").state(CHECKED, self.ui.checked("shortcut-steam-library")));
                            dom.empty(el("span.toggle-switch__slider"));
                        }).clicked() {
                            self.toggle("shortcut-steam-library");
                        }
                    });
                });
                dom.el(self.ui.el("div.completion-message.hidden#shortcut-status"), |dom| {
                    self.fill("shortcut-status", dom);
                });
                dom.el(el("div.card__footer"), |dom| {
                    dom.el(el("div.card__footer-left"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-shortcut-start-over"), |dom| {
                            dom.text("↩");
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("shortcut.goHome"));
                            });
                        }).clicked() {
                            self.click("btn-shortcut-start-over");
                        }
                    });
                    dom.el(el("div.card__footer-right"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-shortcut-skip"), |dom| {
                            self.label("btn-shortcut-skip", "shortcut.skip", dom);
                        }).clicked() {
                            self.click("btn-shortcut-skip");
                        }
                        if dom.button(self.ui.el("button.btn.btn--primary#btn-create-shortcuts"), |dom| {
                            self.label("btn-create-shortcuts", "shortcut.createShortcuts", dom);
                        }).clicked() {
                            self.click("btn-create-shortcuts");
                        }
                    });
                });
            });
        });
    }
}
