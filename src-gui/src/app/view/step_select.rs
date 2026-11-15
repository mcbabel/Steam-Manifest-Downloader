#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_step_select(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.step.hidden#step-select"), |dom| {
            dom.el(el("div.card"), |dom| {
                dom.el(self.ui.el("div.game-info-banner.hidden#game-info-banner"), |dom| {
                    self.image(dom, self.ui.el("img.game-info-banner__image#game-header-image"), "game-header-image");
                    dom.el(el("div.game-info-banner__details"), |dom| {
                        dom.el(self.ui.el("h2.game-info-banner__title#game-name"), |dom| {
                            self.fill("game-name", dom);
                        });
                        dom.el(self.ui.el("p.game-info-banner__description#game-description"), |dom| {
                            self.fill("game-description", dom);
                        });
                    });
                });
                dom.el(self.ui.el("div.loading.hidden#game-info-loading"), |dom| {
                    dom.empty(el("div.spinner"));
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("select.gameInfoLoading"));
                    });
                });
                dom.el(self.ui.el("div.denuvo-note.hidden#denuvo-note"), |dom| {
                    dom.el(el("strong"), |dom| {
                        dom.text(self.t("select.denuvoTitle"));
                    });
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("select.denuvoText"));
                    });
                });
                dom.el(el("div.select-header"), |dom| {
                    dom.el(el("div.select-header__info"), |dom| {
                        dom.el(el("h2"), |dom| {
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("select.appIdLabel"));
                            });
                            dom.el(self.ui.el("span.gradient-text#app-id-display"), |dom| {
                                self.fill("app-id-display", dom);
                            });
                        });
                        dom.el(self.ui.el("p#depot-count"), |dom| {
                            self.fill("depot-count", dom);
                        });
                    });
                    dom.el(el("div.select-header__actions"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.hidden#btn-auto-select"), |dom| {
                            self.label("btn-auto-select", "select.autoSelect", dom);
                        }).clicked() {
                            self.click("btn-auto-select");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.hidden#btn-auto-select-dlc"), |dom| {
                            self.fill("btn-auto-select-dlc", dom);
                        }).clicked() {
                            self.click("btn-auto-select-dlc");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-latest-manifests").title_or(self.t("select.latestAllHint")), |dom| {
                            self.label("btn-latest-manifests", "select.latestAll", dom);
                        }).clicked() {
                            self.click("btn-latest-manifests");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small#btn-select-all"), |dom| {
                            self.label("btn-select-all", "select.selectAll", dom);
                        }).clicked() {
                            self.click("btn-select-all");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-deselect-all"), |dom| {
                            self.label("btn-deselect-all", "select.deselectAll", dom);
                        }).clicked() {
                            self.click("btn-deselect-all");
                        }
                    });
                });
                dom.el(self.ui.el("p.dd-path__hint.latest-manifests-note.hidden#latest-manifests-note"), |dom| {
                    self.fill("latest-manifests-note", dom);
                });
                dom.el(self.ui.el("div.auto-select-note.hidden#auto-select-note"), |dom| {
                    dom.icon_with(el("svg.auto-select-note__icon").size(18.0, 18.0), icons::SPARKLE, 2.0, true);
                    dom.el(el("div.auto-select-note__text"), |dom| {
                        dom.el(self.ui.el("strong#auto-select-title"), |dom| {
                            self.fill("auto-select-title", dom);
                        });
                        dom.el(self.ui.el("span#auto-select-detail"), |dom| {
                            self.fill("auto-select-detail", dom);
                        });
                    });
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline.hidden#btn-auto-start-cancel"), |dom| {
                        self.fill("btn-auto-start-cancel", dom);
                    }).clicked() {
                        self.click("btn-auto-start-cancel");
                    }
                });
                dom.el(el("div.depot-filters"), |dom| {
                    dom.el(el("div.search-box"), |dom| {
                        {
                            let element = self.ui.el("input#depotSearch[type=text]");
                            let placeholder = self.ui.attr("depotSearch", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("select.depotSearchPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("depotSearch"), &placeholder, false);
                            self.input_events("depotSearch", &response);
                        }
                    });
                    if dom.button(el("label.filter-toggle").state(HAS_CHECKED, self.ui.checked("showSelectedOnly")), |dom| {
                        dom.empty(self.ui.el("input#showSelectedOnly[type=checkbox]").state(CHECKED, self.ui.checked("showSelectedOnly")));
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("select.showSelectedOnly"));
                        });
                    }).clicked() {
                        self.toggle("showSelectedOnly");
                    }
                });
                dom.el(self.ui.el("div.steam-keys-offer.hidden#steam-keys-offer"), |dom| {
                    self.fill("steam-keys-offer", dom);
                });
                dom.el(self.ui.el("div.depot-list#depot-list"), |dom| {
                    self.fill("depot-list", dom);
                });
                dom.el(self.ui.el("div.settings-section.branch-section.hidden#branch-section"), |dom| {
                    dom.el(self.ui.el("span.settings-section__label#branch-label"), |dom| {
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("branch.label"));
                        });
                    });
                    dom.el(self.ui.el("div.branch-section__row#branch-row"), |dom| {
                        dom.el(self.ui.el("div.history-sort.branch-picker#branch-picker"), |dom| {
                            if dom.button(self.ui.el("button.history-sort__button.branch-picker__button#branch-button"), |dom| {
                                dom.el(self.ui.el("span.history-sort__label.branch-picker__label#branch-button-label"), |dom| {
                                    self.fill("branch-button-label", dom);
                                });
                                dom.icon_with(el("svg.history-sort__chevron").size(14.0, 14.0), icons::CHEVRON_DOWN, 2.0, true);
                            }).clicked() {
                                self.click("branch-button");
                            }
                            dom.el(self.ui.el("ul.history-sort__menu.history-sort__menu--scroll.branch-picker__menu.hidden#branch-menu"), |dom| {
                                self.fill("branch-menu", dom);
                            });
                        });
                        {
                            let element = self.ui.el("input.dd-path__input.branch-section__password.hidden#branch-password[type=password]");
                            let placeholder = self.ui.attr("branch-password", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("branch.passwordPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("branch-password"), &placeholder, true);
                            self.input_events("branch-password", &response);
                        }
                    });
                    dom.el(self.ui.el("div.branch-section__login.hidden#branch-login-row"), |dom| {
                        dom.el(self.ui.el("span.dd-path__hint#branch-more"), |dom| {
                            self.fill("branch-more", dom);
                        });
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-branch-login"), |dom| {
                            self.label("btn-branch-login", "account.signIn", dom);
                        }).clicked() {
                            self.click("btn-branch-login");
                        }
                    });
                    dom.el(self.ui.el("div.settings-field__toggle-wrap.branch-section__account.hidden#use-account-row"), |dom| {
                        if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("use-account-toggle")), |dom| {
                            dom.empty(self.ui.el("input#use-account-toggle[type=checkbox]").state(CHECKED, self.ui.checked("use-account-toggle")));
                            dom.empty(el("span.toggle-switch__slider"));
                        }).clicked() {
                            self.toggle("use-account-toggle");
                        }
                        if dom.button(el("label.settings-field__toggle-text[for=use-account-toggle]"), |dom| {
                            dom.text(self.t("branch.useAccount"));
                        }).clicked() {
                            self.label_click("use-account-toggle");
                        }
                    });
                    dom.el(self.ui.el("p.dd-path__hint#branch-hint"), |dom| {
                        self.fill("branch-hint", dom);
                    });
                });
                dom.el(el("div.settings-section"), |dom| {
                    if dom.button(el("label.settings-section__label[for=download-dir]"), |dom| {
                        dom.icon_with(el("svg.label-icon").size(16.0, 16.0), icons::FOLDER, 2.0, true);
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("select.downloadLocation"));
                        });
                    }).clicked() {
                        self.label_click("download-dir");
                    }
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#download-dir[type=text]");
                            let placeholder = self.ui.attr("download-dir", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("select.downloadLocationPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("download-dir"), &placeholder, false);
                            self.input_events("download-dir", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.dd-path__browse-btn#btn-browse-dir").title_or(self.t("select.browseFolder")), |dom| {
                            dom.icon_with(el("svg.btn-icon"), icons::FOLDER_OPEN, 2.0, true);
                        }).clicked() {
                            self.click("btn-browse-dir");
                        }
                    });
                    dom.el(el("p.dd-path__hint"), |dom| {
                        dom.text(self.t("select.downloadLocationHint"));
                    });
                });
                dom.el(el("div.settings-section"), |dom| {
                    if dom.button(el("label.settings-section__label[for=mh-apikey]"), |dom| {
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("select.manifestHubLabel"));
                        });
                        dom.el(el("span.optional-badge"), |dom| {
                            dom.text(self.t("common.optional"));
                        });
                    }).clicked() {
                        self.label_click("mh-apikey");
                    }
                    dom.el(el("div.dd-path__input-wrap"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#mh-apikey[type=text]");
                            let placeholder = self.ui.attr("mh-apikey", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("select.manifestHubPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("mh-apikey"), &placeholder, false);
                            self.input_events("mh-apikey", &response);
                        }
                    });
                    dom.el(el("p.dd-path__hint"), |dom| {
                        self.html_key("select.manifestHubHint", dom);
                    });
                    self.fill("mh-apikey-required", dom);
                });
                dom.el(self.ui.el("div.update-notice.hidden#update-mode-notice"), |dom| {
                    self.fill("update-mode-notice", dom);
                });
                dom.el(el("div.card__footer"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-back"), |dom| {
                        dom.text("←");
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("common.back"));
                        });
                    }).clicked() {
                        self.click("btn-back");
                    }
                    dom.el(el("div.card__footer-right"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-queue-add"), |dom| {
                            self.label("btn-queue-add", "queue.add", dom);
                        }).clicked() {
                            self.click("btn-queue-add");
                        }
                        if dom.button(self.ui.el("button.btn.btn--primary#btn-download"), |dom| {
                            if !self.override_content("btn-download", dom) {
                                dom.el(el("span"), |dom| {
                                    dom.text(self.t("select.downloadSelected"));
                                });
                                dom.icon_with(el("svg.btn__icon"), icons::DOWNLOAD, 2.0, false);
                            }
                        }).clicked() {
                            self.click("btn-download");
                        }
                    });
                });
            });
        });
    }
}
