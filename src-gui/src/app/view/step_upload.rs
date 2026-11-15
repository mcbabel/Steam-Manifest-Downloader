#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_step_upload(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.step.active#step-upload"), |dom| {
            dom.el(el("div.card"), |dom| {
                dom.el(el("div.tab-bar"), |dom| {
                    if dom.button(self.ui.el("button.tab-bar__tab.active#tab-upload"), |dom| {
                        dom.icon_with(el("svg").size(16.0, 16.0), icons::FOLDER, 2.0, true);
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("tabs.upload"));
                        });
                    }).clicked() {
                        self.click("tab-upload");
                    }
                    if dom.button(self.ui.el("button.tab-bar__tab#tab-search"), |dom| {
                        dom.icon_with(el("svg").size(16.0, 16.0), icons::SEARCH, 2.0, true);
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("tabs.search"));
                        });
                    }).clicked() {
                        self.click("tab-search");
                    }
                    if dom.button(self.ui.el("button.tab-bar__tab#tab-patch-only"), |dom| {
                        dom.icon_with(el("svg").size(16.0, 16.0), icons::WRENCH, 2.0, true);
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("tabs.patchOnly"));
                        });
                    }).clicked() {
                        self.click("tab-patch-only");
                    }
                });
                self.view_tab_content_upload(dom);
                self.view_tab_content_search(dom);
                self.view_tab_content_patch_only(dom);
            });
        });
    }

    pub(crate) fn view_tab_content_upload(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.tab-content.active#tab-content-upload"), |dom| {
            if dom.button(self.ui.el("div.drop-zone#drop-zone"), |dom| {
                dom.icon_with(el("svg.drop-zone__icon"), icons::UPLOAD, 1.5, false);
                dom.el(el("p.drop-zone__text"), |dom| {
                    self.html_key("upload.dropText", dom);
                });
                dom.el(el("p.drop-zone__hint"), |dom| {
                    dom.text(self.t("upload.dropHint"));
                });
            }).clicked() {
                self.click("drop-zone");
            }
            dom.el(self.ui.el("div.file-info.hidden#file-info"), |dom| {
                dom.icon_with(el("svg.file-info__icon"), icons::FILE, 2.0, false);
                dom.el(self.ui.el("span.file-info__name#file-name"), |dom| {
                    self.fill("file-name", dom);
                });
                if dom.button(self.ui.el("button.file-info__remove#file-remove").title_or(self.t("upload.removeFile")), |dom| {
                    self.fill("file-remove", dom);
                }).clicked() {
                    self.click("file-remove");
                }
            });
            dom.el(self.ui.el("div.error-message.hidden#upload-error"), |dom| {
                self.fill("upload-error", dom);
            });
            dom.el(self.ui.el("div.next-steps.hidden#upload-choice"), |dom| {
                self.fill("upload-choice", dom);
            });
            dom.el(self.ui.el("div.loading.hidden#upload-loading"), |dom| {
                dom.empty(el("div.spinner"));
                dom.el(el("span"), |dom| {
                    dom.text(self.t("upload.parsing"));
                });
            });
        });
    }

    pub(crate) fn view_tab_content_search(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.tab-content#tab-content-search"), |dom| {
            dom.el(self.ui.el("div.sources-empty.hidden#sources-empty-state"), |dom| {
                dom.el(el("h3.sources-empty__title"), |dom| {
                    dom.text(self.t("search.noSourceTitle"));
                });
                dom.el(el("p.sources-empty__lead"), |dom| {
                    self.html_key("search.noSourceLead", dom);
                });
                dom.el(el("div.sources-empty__row"), |dom| {
                    {
                        let element = self.ui.el("input.search-input#sources-empty-input[type=text]");
                        let placeholder = self.ui.attr("sources-empty-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("search.sourcePlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("sources-empty-input"), &placeholder, false);
                        self.input_events("sources-empty-input", &response);
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-sources-empty-add"), |dom| {
                        self.label("btn-sources-empty-add", "search.addSource", dom);
                    }).clicked() {
                        self.click("btn-sources-empty-add");
                    }
                });
                dom.el(self.ui.el("p.error-message.hidden#sources-empty-error"), |dom| {
                    self.fill("sources-empty-error", dom);
                });
                dom.el(el("p.sources-empty__alt"), |dom| {
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("search.noSourceAlt"));
                    });
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-sources-empty-upload"), |dom| {
                        self.label("btn-sources-empty-upload", "search.useUpload", dom);
                    }).clicked() {
                        self.click("btn-sources-empty-upload");
                    }
                });
            });
            dom.el(self.ui.el("div.game-info-banner.hidden#search-game-banner"), |dom| {
                self.image(dom, self.ui.el("img.game-info-banner__image#search-game-image"), "search-game-image");
                dom.el(el("div.game-info-banner__details"), |dom| {
                    dom.el(self.ui.el("h2.game-info-banner__title#search-game-name"), |dom| {
                        self.fill("search-game-name", dom);
                    });
                    dom.el(self.ui.el("p.game-info-banner__description#search-game-description"), |dom| {
                        self.fill("search-game-description", dom);
                    });
                });
            });
            dom.el(self.ui.el("div.search-input-row.hidden#search-input-block"), |dom| {
                dom.el(el("div.search-input-wrapper"), |dom| {
                    {
                        let element = self.ui.el("input.search-input#search-appid-input[type=text]");
                        let placeholder = self.ui.attr("search-appid-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("search.inputPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("search-appid-input"), &placeholder, false);
                        self.input_events("search-appid-input", &response);
                    }
                    dom.el(self.ui.el("div.search-autocomplete.hidden#search-autocomplete"), |dom| {
                        self.fill("search-autocomplete", dom);
                    });
                });
                if dom.button(self.ui.el("button.btn.btn--primary#btn-search"), |dom| {
                    dom.icon_with(el("svg").size(16.0, 16.0), icons::SEARCH, 2.0, false);
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("search.submit"));
                    });
                }).clicked() {
                    self.click("btn-search");
                }
            });
            dom.el(self.ui.el("div.error-message.hidden#search-error"), |dom| {
                self.fill("search-error", dom);
            });
            dom.el(self.ui.el("div.next-steps.hidden#search-next-steps"), |dom| {
                self.fill("search-next-steps", dom);
            });
            dom.el(self.ui.el("div.loading.hidden#search-loading"), |dom| {
                dom.empty(el("div.spinner"));
                dom.el(el("span"), |dom| {
                    dom.text(self.t("search.loading"));
                });
            });
            dom.el(self.ui.el("div.search-results.hidden#search-results"), |dom| {
                dom.el(self.ui.el("div.repo-list#repo-list"), |dom| {
                    self.fill("repo-list", dom);
                });
            });
            dom.el(self.ui.el("div.search-next-row.hidden#search-next-row"), |dom| {
                if dom.button(self.ui.el("button.btn.btn--primary#btn-search-next"), |dom| {
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("search.next"));
                    });
                    dom.text("→");
                }).clicked() {
                    self.click("btn-search-next");
                }
            });
            dom.el(self.ui.el("div.search-account-row.hidden#search-account-row"), |dom| {
                dom.el(el("span.search-account-row__text"), |dom| {
                    dom.text(self.t("account.searchHint"));
                });
                if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-search-account"), |dom| {
                    self.label("btn-search-account", "account.loadFromAccount", dom);
                }).clicked() {
                    self.click("btn-search-account");
                }
            });
            dom.el(self.ui.el("div.search-account-row.search-playtest-row.hidden#search-playtest-row"), |dom| {
                self.fill("search-playtest-row", dom);
            });
            dom.el(self.ui.el("div.loading.hidden#manifest-loading"), |dom| {
                dom.empty(el("div.spinner"));
                dom.el(el("span"), |dom| {
                    dom.text(self.t("search.manifestLoading"));
                });
            });
        });
    }

    pub(crate) fn view_tab_content_patch_only(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.tab-content#tab-content-patch-only"), |dom| {
            dom.el(el("p.patch-only__lead"), |dom| {
                dom.text(self.t("patchOnly.lead"));
            });
            dom.el(el("div.settings-section"), |dom| {
                if dom.button(el("label.settings-section__label[for=patch-only-dir]"), |dom| {
                    dom.icon_with(el("svg.label-icon").size(16.0, 16.0), icons::FOLDER, 2.0, true);
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("patchOnly.folderLabel"));
                    });
                }).clicked() {
                    self.label_click("patch-only-dir");
                }
                dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                    {
                        let element = self.ui.el("input.dd-path__input#patch-only-dir[type=text]");
                        let placeholder = self.ui.attr("patch-only-dir", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("patchOnly.folderPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("patch-only-dir"), &placeholder, false);
                        self.input_events("patch-only-dir", &response);
                    }
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline.dd-path__browse-btn#btn-patch-only-browse").title_or(self.t("patchOnly.browseFolder")), |dom| {
                        dom.icon_with(el("svg.btn-icon"), icons::FOLDER_OPEN, 2.0, true);
                    }).clicked() {
                        self.click("btn-patch-only-browse");
                    }
                });
                dom.el(el("p.dd-path__hint"), |dom| {
                    dom.text(self.t("patchOnly.folderHint"));
                });
            });
            dom.el(el("div.settings-section"), |dom| {
                if dom.button(el("label.settings-section__label[for=patch-only-appid]"), |dom| {
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("patchOnly.appIdLabel"));
                    });
                }).clicked() {
                    self.label_click("patch-only-appid");
                }
                dom.el(el("div.search-input-wrapper"), |dom| {
                    {
                        let element = self.ui.el("input.search-input#patch-only-appid[type=text]");
                        let placeholder = self.ui.attr("patch-only-appid", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("patchOnly.appIdPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("patch-only-appid"), &placeholder, false);
                        self.input_events("patch-only-appid", &response);
                    }
                    dom.el(self.ui.el("div.search-autocomplete.hidden#patch-only-autocomplete"), |dom| {
                        self.fill("patch-only-autocomplete", dom);
                    });
                });
                dom.el(el("p.dd-path__hint"), |dom| {
                    dom.text(self.t("patchOnly.appIdHint"));
                });
            });
            dom.el(self.ui.el("div.error-message.hidden#patch-only-error"), |dom| {
                self.fill("patch-only-error", dom);
            });
            dom.el(self.ui.el("div.loading.hidden#patch-only-loading"), |dom| {
                dom.empty(el("div.spinner"));
                dom.el(el("span"), |dom| {
                    dom.text(self.t("patchOnly.scanning"));
                });
            });
            dom.el(el("div.search-next-row"), |dom| {
                if dom.button(self.ui.el("button.btn.btn--primary#btn-patch-only-start"), |dom| {
                    dom.el(el("span"), |dom| {
                        dom.text(self.t("patchOnly.submit"));
                    });
                    dom.text("→");
                }).clicked() {
                    self.click("btn-patch-only-start");
                }
            });
        });
    }
}
