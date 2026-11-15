#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_step_progress(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.step.hidden#step-progress"), |dom| {
            dom.el(el("div.card"), |dom| {
                dom.el(self.ui.el("div.native-alpha-banner.hidden#native-alpha-banner"), |dom| {
                    dom.el(el("strong"), |dom| {
                        dom.text(self.t("progress.nativeAlphaTitle"));
                    });
                    dom.el(el("p"), |dom| {
                        self.html_key("progress.nativeAlphaBody", dom);
                    });
                });
                dom.el(self.ui.el("div.disk-space-info.hidden#disk-space-info"), |dom| {
                    dom.icon_with(el("svg").size(16.0, 16.0), icons::HARD_DRIVE, 2.0, true);
                    dom.el(self.ui.el("span#disk-space-text"), |dom| {
                        self.label("disk-space-text", "progress.diskSpaceLoading", dom);
                    });
                });
                dom.el(self.ui.el("div.progress-header#progress-header"), |dom| {
                    dom.el(el("h2"), |dom| {
                        dom.text(self.t("progress.downloading"));
                    });
                    dom.el(el("div.progress-bar"), |dom| {
                        dom.el(self.ui.el("div.progress-bar__fill#progress-bar-fill"), |dom| {
                            self.fill("progress-bar-fill", dom);
                        });
                    });
                    dom.el(self.ui.el("p.progress-status#progress-status"), |dom| {
                        self.fill("progress-status", dom);
                    });
                });
                dom.el(self.ui.el("div.depot-progress-list#depot-progress-list"), |dom| {
                    self.fill("depot-progress-list", dom);
                });
                dom.el(el("div.depot-download-progress"), |dom| {
                    dom.el(el("div.depot-progress-bar"), |dom| {
                        dom.el(self.ui.el("div.depot-progress-fill#depot-progress-fill"), |dom| {
                            self.fill("depot-progress-fill", dom);
                        });
                    });
                    dom.el(self.ui.el("span.depot-progress-text#depot-progress-text"), |dom| {
                        self.fill("depot-progress-text", dom);
                    });
                });
                dom.el(self.ui.el("div.download-speed-info.hidden#download-speed-info"), |dom| {
                    dom.el(self.ui.el("span.download-speed-info__speed#download-speed"), |dom| {
                        self.fill("download-speed", dom);
                    });
                    dom.el(el("span.download-speed-info__separator"), |dom| {
                        dom.text("—");
                    });
                    dom.el(self.ui.el("span.download-speed-info__eta#download-eta"), |dom| {
                        self.fill("download-eta", dom);
                    });
                });
                dom.el(el("div.settings-field__toggle-wrap.shutdown-after"), |dom| {
                    if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("shutdown-after-toggle")), |dom| {
                        dom.empty(self.ui.el("input#shutdown-after-toggle[type=checkbox]").state(CHECKED, self.ui.checked("shutdown-after-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    }).clicked() {
                        self.toggle("shutdown-after-toggle");
                    }
                    if dom.button(el("label.settings-field__toggle-text[for=shutdown-after-toggle]"), |dom| {
                        dom.text(self.t("progress.shutdownAfter"));
                    }).clicked() {
                        self.label_click("shutdown-after-toggle");
                    }
                });
                dom.el(self.ui.el("div.terminal#terminal"), |dom| {
                    dom.el(el("div.terminal__header"), |dom| {
                        dom.empty(el("span.terminal__dot.terminal__dot--red"));
                        dom.empty(el("span.terminal__dot.terminal__dot--yellow"));
                        dom.empty(el("span.terminal__dot.terminal__dot--green"));
                        dom.el(el("span.terminal__title"), |dom| {
                            dom.text(self.t("progress.terminalTitle"));
                        });
                    });
                    dom.el(self.ui.el("div.terminal__body#terminal-output"), |dom| {
                        self.fill("terminal-output", dom);
                    });
                });
                dom.el(self.ui.el("div.completion-message.hidden#completion-message"), |dom| {
                    self.fill("completion-message", dom);
                });
                dom.el(self.ui.el("div.next-steps.hidden#failure-advice"), |dom| {
                    self.fill("failure-advice", dom);
                });
                dom.el(self.ui.el("div.feedback-card.hidden#feedback-prompt"), |dom| {
                    dom.el(el("div.feedback-card__icon"), |dom| {
                        dom.icon_with(el("svg").size(20.0, 20.0), icons::STAR, 2.0, true);
                    });
                    dom.el(el("div.feedback-card__text"), |dom| {
                        dom.el(el("div.feedback-card__title"), |dom| {
                            dom.text(self.t("feedback.title"));
                        });
                        dom.el(el("div.feedback-card__hint"), |dom| {
                            dom.text(self.t("feedback.text"));
                        });
                        dom.el(el("div.feedback-card__actions"), |dom| {
                            if dom.button(el("button.btn.btn--small.btn--primary"), |dom| {
                                dom.text(self.t("feedback.star"));
                            }).clicked() {
                                self.click("btn[data-feedback=star]");
                            }
                            if dom.button(el("button.btn.btn--small.btn--outline"), |dom| {
                                dom.text(self.t("feedback.issue"));
                            }).clicked() {
                                self.click("btn[data-feedback=issue]");
                            }
                        });
                    });
                    if dom.button(el("button.feedback-card__close").title_or(self.t("feedback.dismiss")), |dom| {
                        dom.text("×");
                    }).clicked() {
                        self.click("feedback-card__close[data-feedback=dismiss]");
                    }
                });
                dom.el(el("div.card__footer"), |dom| {
                    dom.el(el("div.card__footer-left"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--danger#btn-cancel"), |dom| {
                            if !self.override_content("btn-cancel", dom) {
                                dom.icon_with(el("svg.btn-icon"), icons::X, 2.0, true);
                                dom.el(el("span"), |dom| {
                                    dom.text(self.t("progress.cancel"));
                                });
                            }
                        }).clicked() {
                            self.click("btn-cancel");
                        }
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-pause"), |dom| {
                            self.label("btn-pause", "progress.pause", dom);
                        }).clicked() {
                            self.click("btn-pause");
                        }
                    });
                    dom.el(el("div.card__footer-right"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline.hidden#btn-report-failure"), |dom| {
                            self.label("btn-report-failure", "bugReport.reportThis", dom);
                        }).clicked() {
                            self.click("btn-report-failure");
                        }
                        if dom.button(self.ui.el("button.btn.btn--primary.hidden#btn-next-step"), |dom| {
                            self.label("btn-next-step", "progress.next", dom);
                        }).clicked() {
                            self.click("btn-next-step");
                        }
                    });
                });
            });
        });
    }
}
