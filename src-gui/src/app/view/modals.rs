#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_cancel_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#cancel-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("cancel-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#cancel-modal-title"), |dom| {
                    self.label("cancel-modal-title", "modals.cancel.title", dom);
                });
                dom.el(self.ui.el("p.modal__text#cancel-modal-body"), |dom| {
                    self.label_html("cancel-modal-body", "modals.cancel.body", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-cancel-no"), |dom| {
                        self.label("btn-cancel-no", "modals.cancel.no", dom);
                    }).clicked() {
                        self.click("btn-cancel-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-cancel-yes"), |dom| {
                        self.label("btn-cancel-yes", "modals.cancel.yes", dom);
                    }).clicked() {
                        self.click("btn-cancel-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_shutdown_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#shutdown-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("shutdown-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::POWER, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#shutdown-modal-title"), |dom| {
                    self.label("shutdown-modal-title", "modals.shutdown.title", dom);
                });
                dom.el(self.ui.el("p.modal__text#shutdown-modal-body"), |dom| {
                    self.fill("shutdown-modal-body", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-shutdown-abort"), |dom| {
                        self.label("btn-shutdown-abort", "modals.shutdown.abort", dom);
                    }).clicked() {
                        self.click("btn-shutdown-abort");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-shutdown-now"), |dom| {
                        self.label("btn-shutdown-now", "modals.shutdown.now", dom);
                    }).clicked() {
                        self.click("btn-shutdown-now");
                    }
                });
            });
        });
    }

    pub(crate) fn view_followup_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#followup-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("followup-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--accent"), |dom| {
                    dom.icon_with(el("svg"), icons::CHEVRON_RIGHT, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#followup-modal-title"), |dom| {
                    self.label("followup-modal-title", "modals.followup.title", dom);
                });
                dom.el(self.ui.el("p.modal__text#followup-modal-body"), |dom| {
                    self.fill("followup-modal-body", dom);
                });
                dom.el(el("div.modal__actions.modal__actions--stack"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-followup-later"), |dom| {
                        self.label("btn-followup-later", "modals.followup.later", dom);
                    }).clicked() {
                        self.click("btn-followup-later");
                    }
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-followup-discard"), |dom| {
                        self.label("btn-followup-discard", "modals.followup.discard", dom);
                    }).clicked() {
                        self.click("btn-followup-discard");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-followup-resume"), |dom| {
                        self.label("btn-followup-resume", "modals.followup.resume", dom);
                    }).clicked() {
                        self.click("btn-followup-resume");
                    }
                });
            });
        });
    }

    pub(crate) fn view_close_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#close-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("close-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#close-modal-title"), |dom| {
                    self.label("close-modal-title", "modals.close.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    self.html_key("modals.close.body", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-close-no"), |dom| {
                        self.label("btn-close-no", "modals.close.no", dom);
                    }).clicked() {
                        self.click("btn-close-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-close-yes"), |dom| {
                        self.label("btn-close-yes", "modals.close.yes", dom);
                    }).clicked() {
                        self.click("btn-close-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_remove_source_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#remove-source-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("remove-source-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#remove-source-modal-title"), |dom| {
                    self.label("remove-source-modal-title", "modals.removeDefaultSource.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    self.html_key("modals.removeDefaultSource.body", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-remove-source-no"), |dom| {
                        self.label("btn-remove-source-no", "modals.removeDefaultSource.no", dom);
                    }).clicked() {
                        self.click("btn-remove-source-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-remove-source-yes"), |dom| {
                        self.label("btn-remove-source-yes", "modals.removeDefaultSource.yes", dom);
                    }).clicked() {
                        self.click("btn-remove-source-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_history_remove_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#history-remove-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("history-remove-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::TRASH_MODAL, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#history-remove-modal-title"), |dom| {
                    self.label("history-remove-modal-title", "modals.historyRemove.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    self.label_html("history-remove-body", "modals.historyRemove.body", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-history-remove-no"), |dom| {
                        self.label("btn-history-remove-no", "modals.historyRemove.no", dom);
                    }).clicked() {
                        self.click("btn-history-remove-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-history-remove-yes"), |dom| {
                        self.label("btn-history-remove-yes", "modals.historyRemove.yes", dom);
                    }).clicked() {
                        self.click("btn-history-remove-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_emu_revert_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#emu-revert-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("emu-revert-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#emu-revert-modal-title"), |dom| {
                    self.label("emu-revert-modal-title", "modals.emuRevert.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    self.html_key("modals.emuRevert.body", dom);
                });
                dom.el(self.ui.el("p.modal__text#emu-revert-scope"), |dom| {
                    self.fill("emu-revert-scope", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-emu-revert-no"), |dom| {
                        self.label("btn-emu-revert-no", "modals.emuRevert.no", dom);
                    }).clicked() {
                        self.click("btn-emu-revert-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-emu-revert-yes"), |dom| {
                        self.label("btn-emu-revert-yes", "modals.emuRevert.yes", dom);
                    }).clicked() {
                        self.click("btn-emu-revert-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_emu_download_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#emu-download-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("emu-download-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon"), |dom| {
                    dom.icon_with(el("svg"), icons::DOWNLOAD, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#emu-download-modal-title"), |dom| {
                    self.label("emu-download-modal-title", "modals.emuDownload.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    self.html_key("modals.emuDownload.body", dom);
                });
                dom.el(self.ui.el("p.modal__text#emu-download-scope"), |dom| {
                    self.fill("emu-download-scope", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-emu-download-no"), |dom| {
                        self.label("btn-emu-download-no", "modals.emuDownload.no", dom);
                    }).clicked() {
                        self.click("btn-emu-download-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-emu-download-yes"), |dom| {
                        self.label("btn-emu-download-yes", "modals.emuDownload.yes", dom);
                    }).clicked() {
                        self.click("btn-emu-download-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_history_clear_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#history-clear-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("history-clear-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#history-clear-modal-title"), |dom| {
                    self.label("history-clear-modal-title", "modals.historyClear.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    self.label_html("history-clear-body", "modals.historyClear.body", dom);
                });
                dom.el(self.ui.el("div.modal__notice.hidden#history-clear-resumable-warning"), |dom| {
                    dom.el(self.ui.el("p.modal__text#history-clear-resumable-text"), |dom| {
                        self.fill("history-clear-resumable-text", dom);
                    });
                    if dom.button(el("label.modal__check").state(HAS_CHECKED, self.ui.checked("history-clear-delete-files")), |dom| {
                        dom.empty(self.ui.el("input#history-clear-delete-files[type=checkbox]").state(CHECKED, self.ui.checked("history-clear-delete-files")));
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("modals.historyClear.deleteFilesLabel"));
                        });
                    }).clicked() {
                        self.toggle("history-clear-delete-files");
                    }
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-history-clear-no"), |dom| {
                        self.label("btn-history-clear-no", "modals.historyClear.no", dom);
                    }).clicked() {
                        self.click("btn-history-clear-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-history-clear-yes"), |dom| {
                        self.label("btn-history-clear-yes", "modals.historyClear.yes", dom);
                    }).clicked() {
                        self.click("btn-history-clear-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_telemetry_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#telemetry-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("telemetry-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.telemetry-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#telemetry-modal-title"), |dom| {
                    dom.icon_with(el("svg.modal__title-icon").size(20.0, 20.0), icons::BAR_CHART, 2.0, true);
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("modals.telemetry.title"));
                    });
                });
                dom.el(el("p.telemetry-modal__lead"), |dom| {
                    self.html_key("modals.telemetry.lead", dom);
                });
                dom.el(el("div.telemetry-modal__list"), |dom| {
                    dom.el(el("div.telemetry-modal__col.telemetry-modal__col--collect"), |dom| {
                        dom.el(el("h4"), |dom| {
                            dom.icon_with(el("svg"), icons::CHECK_CIRCLE, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("modals.telemetry.collectTitle"));
                            });
                        });
                        dom.el(el("ul"), |dom| {
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.collectItem1"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.collectItem2"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.collectItem3"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.collectItem4"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.collectItem5"));
                            });
                        });
                    });
                    dom.el(el("div.telemetry-modal__col.telemetry-modal__col--never"), |dom| {
                        dom.el(el("h4"), |dom| {
                            dom.icon_with(el("svg"), icons::SHIELD_OFF, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("modals.telemetry.neverTitle"));
                            });
                        });
                        dom.el(el("ul"), |dom| {
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.neverItem1"));
                            });
                            dom.el(el("li"), |dom| {
                                self.html_key("modals.telemetry.neverItem2", dom);
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.neverItem3"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.telemetry.neverItem4"));
                            });
                        });
                    });
                });
                dom.el(el("p.dd-path__hint"), |dom| {
                    dom.text(self.t("modals.telemetry.hint"));
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-telemetry-decline"), |dom| {
                        self.label("btn-telemetry-decline", "modals.telemetry.decline", dom);
                    }).clicked() {
                        self.click("btn-telemetry-decline");
                    }
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-telemetry-basic"), |dom| {
                        self.label("btn-telemetry-basic", "modals.telemetry.basic", dom);
                    }).clicked() {
                        self.click("btn-telemetry-basic");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-telemetry-accept"), |dom| {
                        self.label("btn-telemetry-accept", "modals.telemetry.accept", dom);
                    }).clicked() {
                        self.click("btn-telemetry-accept");
                    }
                });
            });
        });
    }

    pub(crate) fn view_behavior_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#behavior-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("behavior-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.telemetry-modal-content.behavior-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#behavior-modal-title"), |dom| {
                    dom.icon_with(el("svg.modal__title-icon").size(20.0, 20.0), icons::POINTER, 2.0, true);
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("modals.behavior.title"));
                    });
                });
                dom.el(el("p.telemetry-modal__lead"), |dom| {
                    dom.text(self.t("modals.behavior.lead"));
                });
                dom.el(el("div.telemetry-modal__list"), |dom| {
                    dom.el(el("div.telemetry-modal__col.telemetry-modal__col--collect"), |dom| {
                        dom.el(el("h4"), |dom| {
                            dom.icon_with(el("svg"), icons::CHECK_CIRCLE, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("modals.telemetry.collectTitle"));
                            });
                        });
                        dom.el(el("ul"), |dom| {
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.behavior.item1"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.behavior.item2"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.behavior.item3"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.behavior.item4"));
                            });
                        });
                    });
                    dom.el(el("div.telemetry-modal__col.telemetry-modal__col--never"), |dom| {
                        dom.el(el("h4"), |dom| {
                            dom.icon_with(el("svg"), icons::SHIELD_OFF, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("modals.telemetry.neverTitle"));
                            });
                        });
                        dom.el(el("ul"), |dom| {
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.behavior.never1"));
                            });
                            dom.el(el("li"), |dom| {
                                dom.text(self.t("modals.behavior.never2"));
                            });
                        });
                    });
                });
                dom.el(el("p.dd-path__hint.behavior-modal__hint"), |dom| {
                    dom.text(self.t("modals.behavior.hint"));
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-behavior-decline"), |dom| {
                        self.label("btn-behavior-decline", "modals.behavior.decline", dom);
                    }).clicked() {
                        self.click("btn-behavior-decline");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-behavior-accept"), |dom| {
                        self.label("btn-behavior-accept", "modals.behavior.accept", dom);
                    }).clicked() {
                        self.click("btn-behavior-accept");
                    }
                });
            });
        });
    }

    pub(crate) fn view_bug_report_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#bug-report-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("bug-report-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.bug-report"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#bug-report-title"), |dom| {
                    dom.el(el("span.modal__title-icon"), |dom| {
                        dom.icon_with(el("svg").size(16.0, 16.0), icons::BUG, 2.0, true);
                    });
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("bugReport.title"));
                    });
                });
                dom.el(el("p.bug-report__intro"), |dom| {
                    dom.text(self.t("bugReport.intro"));
                });
                dom.el(el("div.bug-report__fields"), |dom| {
                    dom.el(el("label.bug-report__field"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("bugReport.summary"));
                        });
                        {
                            let element = self.ui.el("input.dd-path__input.bug-report__input#bug-title[type=text]");
                            let placeholder = self.ui.attr("bug-title", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("bugReport.summaryPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("bug-title"), &placeholder, false);
                            self.input_events("bug-title", &response);
                        }
                    });
                    dom.el(el("label.bug-report__field"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("bugReport.what"));
                        });
                        {
                            let element = self.ui.el("textarea.dd-path__input.bug-report__input#bug-what");
                            let placeholder = self.t("bugReport.whatPlaceholder").to_string();
                            let response = dom.textarea(element, self.ui.value_mut("bug-what"), &placeholder, 3);
                            self.input_events("bug-what", &response);
                        }
                    });
                    dom.el(el("label.bug-report__field"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("bugReport.steps"));
                        });
                        {
                            let element = self.ui.el("textarea.dd-path__input.bug-report__input#bug-steps");
                            let placeholder = self.t("bugReport.stepsPlaceholder").to_string();
                            let response = dom.textarea(element, self.ui.value_mut("bug-steps"), &placeholder, 2);
                            self.input_events("bug-steps", &response);
                        }
                    });
                    dom.el(el("label.bug-report__field"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("bugReport.expected"));
                        });
                        {
                            let element = self.ui.el("textarea.dd-path__input.bug-report__input#bug-expected");
                            let placeholder = String::new();
                            let response = dom.textarea(element, self.ui.value_mut("bug-expected"), &placeholder, 2);
                            self.input_events("bug-expected", &response);
                        }
                    });
                    dom.el(self.ui.el_as("settings-group-1", "div.settings-group"), |dom| {
                        if dom.button(self.ui.el_as("settings-row-1", "label.settings-row[for=bug-include-diag]").state(HAS_CHECKED, self.ui.checked("bug-include-diag")), |dom| {
                            dom.el(el("span.settings-row__text"), |dom| {
                                dom.el(el("span.settings-row__label"), |dom| {
                                    dom.text(self.t("bugReport.includeDiag"));
                                });
                                dom.el(self.ui.el("span.settings-row__hint#bug-diag-preview"), |dom| {
                                    self.fill("bug-diag-preview", dom);
                                });
                            });
                            dom.el(el("span.toggle-switch"), |dom| {
                                dom.empty(self.ui.el("input#bug-include-diag[type=checkbox]").state(CHECKED, self.ui.checked("bug-include-diag")));
                                dom.empty(el("span.toggle-switch__slider"));
                            });
                        }).clicked() {
                            self.toggle("bug-include-diag");
                        }
                        if dom.button(self.ui.el_as("settings-row-2", "label.settings-row[for=bug-include-log]").state(HAS_CHECKED, self.ui.checked("bug-include-log")), |dom| {
                            dom.el(el("span.settings-row__text"), |dom| {
                                dom.el(el("span.settings-row__label"), |dom| {
                                    dom.text(self.t("bugReport.includeLog"));
                                });
                                dom.el(el("span.settings-row__hint"), |dom| {
                                    dom.text(self.t("bugReport.includeLogHint"));
                                });
                            });
                            dom.el(el("span.toggle-switch"), |dom| {
                                dom.empty(self.ui.el("input#bug-include-log[type=checkbox]").state(CHECKED, self.ui.checked("bug-include-log")));
                                dom.empty(el("span.toggle-switch__slider"));
                            });
                        }).clicked() {
                            self.toggle("bug-include-log");
                        }
                    });
                    dom.el(self.ui.el("p.error-message.hidden#bug-report-error"), |dom| {
                        self.fill("bug-report-error", dom);
                    });
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-bug-cancel"), |dom| {
                        self.label("btn-bug-cancel", "settings.cancel", dom);
                    }).clicked() {
                        self.click("btn-bug-cancel");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-bug-open"), |dom| {
                        self.label("btn-bug-open", "bugReport.open", dom);
                    }).clicked() {
                        self.click("btn-bug-open");
                    }
                });
            });
        });
    }

    pub(crate) fn view_whats_new_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#whats-new-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("whats-new-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.update-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(el("header.update-modal__header"), |dom| {
                    dom.el(el("div.update-modal__icon"), |dom| {
                        dom.icon_with(el("svg").size(22.0, 22.0), icons::STAR_UPDATE, 2.0, true);
                    });
                    dom.el(el("div.update-modal__heading"), |dom| {
                        dom.el(self.ui.el("h3.update-modal__title#whats-new-title"), |dom| {
                            self.fill("whats-new-title", dom);
                        });
                        dom.el(el("div.update-modal__meta"), |dom| {
                            dom.el(self.ui.el("span.update-modal__date#whats-new-date"), |dom| {
                                self.fill("whats-new-date", dom);
                            });
                        });
                    });
                });
                dom.el(el("div.update-modal__body"), |dom| {
                    dom.el(self.ui.el("div.update-modal__notes#whats-new-notes"), |dom| {
                        self.fill("whats-new-notes", dom);
                    });
                });
                dom.el(el("footer.update-modal__actions"), |dom| {
                    dom.el(el("div.update-modal__actions-left"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--small.update-modal__skip#btn-whats-new-github"), |dom| {
                            self.label("btn-whats-new-github", "modals.update.onGithub", dom);
                        }).clicked() {
                            self.click("btn-whats-new-github");
                        }
                    });
                    dom.el(el("div.update-modal__actions-right"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--primary#btn-whats-new-close"), |dom| {
                            self.label("btn-whats-new-close", "whatsNew.close", dom);
                        }).clicked() {
                            self.click("btn-whats-new-close");
                        }
                    });
                });
            });
        });
    }

    pub(crate) fn view_update_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#update-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("update-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.update-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(el("header.update-modal__header"), |dom| {
                    dom.el(el("div.update-modal__icon"), |dom| {
                        dom.icon_with(el("svg").size(22.0, 22.0), icons::DOWNLOAD, 2.0, true);
                    });
                    dom.el(el("div.update-modal__heading"), |dom| {
                        dom.el(self.ui.el("h3.update-modal__title#update-modal-title"), |dom| {
                            self.label("update-modal-title", "modals.update.title", dom);
                        });
                        dom.el(el("div.update-modal__meta"), |dom| {
                            dom.el(el("span.update-modal__versions"), |dom| {
                                dom.el(self.ui.el("span.update-modal__current#update-current").state(crate::style::selector::EMPTY, self.is_empty("update-current")), |dom| {
                                    self.fill("update-current", dom);
                                });
                                dom.el(el("span.update-modal__arrow"), |dom| {
                                    dom.text("→");
                                });
                                dom.el(self.ui.el("span.update-modal__version#update-version"), |dom| {
                                    self.fill("update-version", dom);
                                });
                            });
                            dom.el(self.ui.el("span.update-modal__date#update-date-row"), |dom| {
                                dom.el(el("span.update-modal__dot"), |dom| {
                                    dom.text("·");
                                });
                                dom.el(self.ui.el("span#update-date"), |dom| {
                                    self.fill("update-date", dom);
                                });
                            });
                        });
                    });
                });
                dom.el(el("div.update-modal__body"), |dom| {
                    dom.el(self.ui.el("div.update-modal__notes#update-notes"), |dom| {
                        self.fill("update-notes", dom);
                    });
                });
                dom.el(self.ui.el("div.update-modal__system-hint.hidden#update-system-hint"), |dom| {
                    dom.el(self.ui.el("p#update-system-text"), |dom| {
                        self.label("update-system-text", "modals.update.systemText", dom);
                    });
                    dom.el(self.ui.el("div.update-system-cmd-list#update-system-cmd-list"), |dom| {
                        self.fill("update-system-cmd-list", dom);
                    });
                });
                dom.el(self.ui.el("div.update-modal__progress.hidden#update-progress-wrap"), |dom| {
                    dom.el(el("div.progress-bar"), |dom| {
                        dom.el(self.ui.el("div.progress-bar__fill#update-progress-fill"), |dom| {
                            self.fill("update-progress-fill", dom);
                        });
                    });
                    dom.el(self.ui.el("p.update-modal__progress-text#update-progress-text"), |dom| {
                        self.label("update-progress-text", "modals.update.downloading", dom);
                    });
                });
                dom.el(self.ui.el("footer.update-modal__actions#update-actions"), |dom| {
                    dom.el(el("div.update-modal__actions-left"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--small.update-modal__skip#btn-update-skip"), |dom| {
                            self.label("btn-update-skip", "modals.update.skip", dom);
                        }).clicked() {
                            self.click("btn-update-skip");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.update-modal__skip.hidden#btn-update-notes-link"), |dom| {
                            self.label("btn-update-notes-link", "modals.update.onGithub", dom);
                        }).clicked() {
                            self.click("btn-update-notes-link");
                        }
                    });
                    dom.el(el("div.update-modal__actions-right"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-update-later"), |dom| {
                            self.label("btn-update-later", "modals.update.later", dom);
                        }).clicked() {
                            self.click("btn-update-later");
                        }
                        if dom.button(self.ui.el("button.btn.btn--primary#btn-update-now"), |dom| {
                            self.label("btn-update-now", "modals.update.now", dom);
                        }).clicked() {
                            self.click("btn-update-now");
                        }
                    });
                });
            });
        });
    }

    pub(crate) fn view_language_picker(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#language-picker"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("language-picker>modal__backdrop");
            }
            dom.hit(el("div.modal__content.language-picker-content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--accent"), |dom| {
                    dom.icon_with(el("svg"), icons::GLOBE_LANG, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#language-picker-title"), |dom| {
                    self.label("language-picker-title", "languagePicker.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    dom.text(self.t("languagePicker.subtitle"));
                });
                dom.el(self.ui.el("div.language-cards#language-picker-cards"), |dom| {
                    self.fill("language-picker-cards", dom);
                });
            });
        });
    }

    pub(crate) fn view_welcome_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#welcome-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("welcome-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.welcome-content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--accent"), |dom| {
                    dom.icon_with(el("svg"), icons::DOWNLOAD, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#welcome-title"), |dom| {
                    self.label("welcome-title", "welcome.title", dom);
                });
                dom.el(el("p.modal__text.welcome__lead"), |dom| {
                    dom.text(self.t("welcome.lead"));
                });
                dom.el(el("div.welcome__choices"), |dom| {
                    if dom.button(el("button.welcome__choice"), |dom| {
                        dom.el(el("span.welcome__choice-icon"), |dom| {
                            dom.icon_with(el("svg").size(20.0, 20.0), icons::FOLDER, 2.0, true);
                        });
                        dom.el(el("span.welcome__choice-text"), |dom| {
                            dom.el(el("span.welcome__choice-title"), |dom| {
                                dom.text(self.t("welcome.uploadTitle"));
                            });
                            dom.el(el("span.welcome__choice-hint"), |dom| {
                                dom.text(self.t("welcome.uploadHint"));
                            });
                        });
                    }).clicked() {
                        self.click("welcome__choice[data-choice=upload]");
                    }
                    if dom.button(el("button.welcome__choice"), |dom| {
                        dom.el(el("span.welcome__choice-icon"), |dom| {
                            dom.icon_with(el("svg").size(20.0, 20.0), icons::SEARCH, 2.0, true);
                        });
                        dom.el(el("span.welcome__choice-text"), |dom| {
                            dom.el(el("span.welcome__choice-title"), |dom| {
                                dom.text(self.t("welcome.searchTitle"));
                            });
                            dom.el(el("span.welcome__choice-hint"), |dom| {
                                dom.text(self.t("welcome.searchHint"));
                            });
                        });
                    }).clicked() {
                        self.click("welcome__choice[data-choice=search]");
                    }
                    if dom.button(el("button.welcome__choice"), |dom| {
                        dom.el(el("span.welcome__choice-icon"), |dom| {
                            dom.icon_with(el("svg").size(20.0, 20.0), icons::WRENCH, 2.0, true);
                        });
                        dom.el(el("span.welcome__choice-text"), |dom| {
                            dom.el(el("span.welcome__choice-title"), |dom| {
                                dom.text(self.t("welcome.patchTitle"));
                            });
                            dom.el(el("span.welcome__choice-hint"), |dom| {
                                dom.text(self.t("welcome.patchHint"));
                            });
                        });
                    }).clicked() {
                        self.click("welcome__choice[data-choice=patch]");
                    }
                });
                dom.el(el("div.welcome__location"), |dom| {
                    dom.el(el("span.welcome__location-label"), |dom| {
                        dom.text(self.t("welcome.location"));
                    });
                    dom.el(self.ui.el("code.welcome__location-path#welcome-location-path"), |dom| {
                        self.fill("welcome-location-path", dom);
                    });
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-welcome-location"), |dom| {
                        self.label("btn-welcome-location", "welcome.change", dom);
                    }).clicked() {
                        self.click("btn-welcome-location");
                    }
                });
                dom.el(el("div.welcome__footer"), |dom| {
                    if dom.button(el("a.link"), |dom| {
                        dom.text(self.t("welcome.docs"));
                    }).clicked() {
                        self.open_url("https://smd.mcbabel.de/documentation/");
                    }
                    if dom.button(self.ui.el("button.btn.btn--small#btn-welcome-close"), |dom| {
                        self.label("btn-welcome-close", "welcome.later", dom);
                    }).clicked() {
                        self.click("btn-welcome-close");
                    }
                });
            });
        });
    }

    pub(crate) fn view_history_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#history-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("history-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.history-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#history-modal-title"), |dom| {
                    dom.icon_with(el("svg.modal__title-icon").size(18.0, 18.0), icons::CLOCK, 2.0, false);
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("modals.history.title"));
                    });
                });
                dom.el(self.ui.el("div.history-banner.hidden#history-banner"), |dom| {
                    self.fill("history-banner", dom);
                });
                dom.el(self.ui.el("div.history-toolbar.hidden#history-toolbar"), |dom| {
                    dom.el(el("div.history-toolbar__row"), |dom| {
                        {
                            let element = self.ui.el("input.history-toolbar__search#history-search[type=search]");
                            let placeholder = self.ui.attr("history-search", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("history.searchPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("history-search"), &placeholder, false);
                            self.input_events("history-search", &response);
                        }
                        dom.el(self.ui.el("div.history-sort#history-sort"), |dom| {
                            if dom.button(self.ui.el("button.history-sort__button#history-sort-button"), |dom| {
                                dom.el(self.ui.el("span.history-sort__label#history-sort-label"), |dom| {
                                    self.fill("history-sort-label", dom);
                                });
                                dom.icon_with(el("svg.history-sort__chevron").size(14.0, 14.0), icons::CHEVRON_DOWN, 2.0, true);
                            }).clicked() {
                                self.click("history-sort-button");
                            }
                            dom.el(self.ui.el("ul.history-sort__menu.hidden#history-sort-menu"), |dom| {
                                if dom.button(self.ui.el_as("history-sort-newest", "li.history-sort__option"), |dom| {
                                    dom.text(self.t("history.sortNewest"));
                                }).clicked() {
                                    self.click("history-sort:newest");
                                }
                                if dom.button(self.ui.el_as("history-sort-oldest", "li.history-sort__option"), |dom| {
                                    dom.text(self.t("history.sortOldest"));
                                }).clicked() {
                                    self.click("history-sort:oldest");
                                }
                                if dom.button(self.ui.el_as("history-sort-name", "li.history-sort__option"), |dom| {
                                    dom.text(self.t("history.sortName"));
                                }).clicked() {
                                    self.click("history-sort:name");
                                }
                                if dom.button(self.ui.el_as("history-sort-size", "li.history-sort__option"), |dom| {
                                    dom.text(self.t("history.sortSize"));
                                }).clicked() {
                                    self.click("history-sort:size");
                                }
                            });
                        });
                    });
                    dom.el(self.ui.el("div.history-toolbar__filters#history-filters"), |dom| {
                        self.fill("history-filters", dom);
                    });
                });
                dom.el(self.ui.el("div.history-list#history-list"), |dom| {
                    self.fill("history-list", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline.btn--small.btn--push-end#btn-history-clear"), |dom| {
                        self.label("btn-history-clear", "modals.history.clear", dom);
                    }).clicked() {
                        self.click("btn-history-clear");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-history-close"), |dom| {
                        self.label("btn-history-close", "modals.history.close", dom);
                    }).clicked() {
                        self.click("btn-history-close");
                    }
                });
            });
        });
    }

    pub(crate) fn view_resume_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#resume-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("resume-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--accent"), |dom| {
                    dom.icon_with(el("svg"), icons::PLAY_MODAL, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#resume-modal-title"), |dom| {
                    self.label("resume-modal-title", "modals.resume.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    dom.text(self.t("modals.resume.body"));
                });
                dom.el(el("div.modal__actions.modal__actions--stack"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-resume-cancel"), |dom| {
                        self.label("btn-resume-cancel", "common.cancel", dom);
                    }).clicked() {
                        self.click("btn-resume-cancel");
                    }
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-resume-verify"), |dom| {
                        self.label("btn-resume-verify", "modals.resume.verify", dom);
                    }).clicked() {
                        self.click("btn-resume-verify");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-resume-fast"), |dom| {
                        self.label("btn-resume-fast", "modals.resume.fast", dom);
                    }).clicked() {
                        self.click("btn-resume-fast");
                    }
                });
            });
        });
    }

    pub(crate) fn view_launch_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#launch-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("launch-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.launch-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#launch-modal-title"), |dom| {
                    self.label("launch-modal-title", "modals.launch.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    dom.text(self.t("modals.launch.body"));
                });
                dom.el(self.ui.el("div.launch-list#launch-list"), |dom| {
                    self.fill("launch-list", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-launch-cancel"), |dom| {
                        self.label("btn-launch-cancel", "common.cancel", dom);
                    }).clicked() {
                        self.click("btn-launch-cancel");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-launch-start"), |dom| {
                        self.label("btn-launch-start", "modals.launch.start", dom);
                    }).clicked() {
                        self.click("btn-launch-start");
                    }
                });
            });
        });
    }

    pub(crate) fn view_queue_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#queue-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("queue-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.queue-modal-content"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#queue-modal-title"), |dom| {
                    self.label("queue-modal-title", "queue.title", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    dom.text(self.t("queue.body"));
                });
                dom.el(self.ui.el("div.queue-running.hidden#queue-running"), |dom| {
                    self.fill("queue-running", dom);
                });
                dom.el(self.ui.el("div.queue-list#queue-list"), |dom| {
                    self.fill("queue-list", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline.btn--small.btn--push-end#btn-queue-clear"), |dom| {
                        self.label("btn-queue-clear", "queue.clear", dom);
                    }).clicked() {
                        self.click("btn-queue-clear");
                    }
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-queue-close"), |dom| {
                        self.label("btn-queue-close", "common.close", dom);
                    }).clicked() {
                        self.click("btn-queue-close");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-queue-start"), |dom| {
                        self.label("btn-queue-start", "queue.start", dom);
                    }).clicked() {
                        self.click("btn-queue-start");
                    }
                });
            });
        });
    }

    pub(crate) fn view_confirm_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#confirm-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("confirm-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content"), egui::Sense::CLICK, |dom| {
                dom.el(el("div.modal__icon.modal__icon--warning"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(self.ui.el("h3.modal__title#confirm-modal-title"), |dom| {
                    self.fill("confirm-modal-title", dom);
                });
                dom.el(self.ui.el("p.modal__text#confirm-modal-body"), |dom| {
                    self.fill("confirm-modal-body", dom);
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-confirm-no"), |dom| {
                        self.fill("btn-confirm-no", dom);
                    }).clicked() {
                        self.click("btn-confirm-no");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-confirm-yes"), |dom| {
                        self.fill("btn-confirm-yes", dom);
                    }).clicked() {
                        self.click("btn-confirm-yes");
                    }
                });
            });
        });
    }

    pub(crate) fn view_account_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#account-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("account-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.account-modal"), egui::Sense::CLICK, |dom| {
                dom.el(self.ui.el("h3.modal__title#account-modal-title"), |dom| {
                    self.label("account-modal-title", "account.qrTitle", dom);
                });
                dom.el(el("p.modal__text"), |dom| {
                    dom.text(self.t("account.qrIntro"));
                });
                dom.el(self.ui.el("div.account-qr.account-qr--empty#account-qr"), |dom| {
                    self.fill("account-qr", dom);
                });
                dom.el(self.ui.el("p.account-modal__status#account-modal-status"), |dom| {
                    self.fill("account-modal-status", dom);
                });
                dom.el(el("p.settings-row__hint"), |dom| {
                    dom.text(self.t("account.qrNote"));
                });
                dom.el(el("div.modal__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-account-cancel"), |dom| {
                        self.label("btn-account-cancel", "settings.cancel", dom);
                    }).clicked() {
                        self.click("btn-account-cancel");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary.hidden#btn-account-retry"), |dom| {
                        self.label("btn-account-retry", "account.qrRetry", dom);
                    }).clicked() {
                        self.click("btn-account-retry");
                    }
                });
            });
        });
    }
}
