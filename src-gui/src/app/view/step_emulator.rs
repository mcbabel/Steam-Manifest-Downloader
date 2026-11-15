#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_step_emulator(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.step.hidden#step-emulator"), |dom| {
            dom.el(el("div.card"), |dom| {
                dom.el(el("div.emu-top"), |dom| {
                    dom.el(el("div.emu-top__text"), |dom| {
                        dom.el(el("h2.emu-header"), |dom| {
                            self.label("emu-header", "emulator.title", dom);
                        });
                        dom.el(el("p.emu-description"), |dom| {
                            self.label("emu-description", "emulator.description", dom);
                        });
                    });
                    dom.el(self.ui.el("div.emu-release-status#emu-release-status"), |dom| {
                        self.label("emu-release-status", "emulator.releaseLoading", dom);
                    });
                });
                dom.el(self.ui.el("div.emu-summary#emu-summary").state(crate::style::selector::EMPTY, self.is_empty("emu-summary")), |dom| {
                    self.fill("emu-summary", dom);
                });
                dom.el(el("div.emu-body"), |dom| {
                    dom.el(el("nav.emu-nav"), |dom| {
                        if dom.button(self.ui.el("button.emu-nav__tab.is-active#emu-tab-files[data-section=files]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::FILE, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.files"));
                            });
                            dom.el(self.ui.el_as("emu-badge-files", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-files", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-files");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-gamedata[data-section=gamedata]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::AWARD, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.gameData"));
                            });
                            dom.el(self.ui.el_as("emu-badge-gamedata", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-gamedata", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-gamedata");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-controller[data-section=controller]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::GAMEPAD_TAB, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.controller"));
                            });
                            dom.el(self.ui.el_as("emu-badge-controller", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-controller", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-controller");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-emulator[data-section=emulator]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::SLIDERS, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.emulator"));
                            });
                            dom.el(self.ui.el_as("emu-badge-emulator", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-emulator", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-emulator");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-saves[data-section=saves]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::SAVE, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.saves"));
                            });
                            dom.el(self.ui.el_as("emu-badge-saves", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-saves", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-saves");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-profile[data-section=profile]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::USER, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.profile"));
                            });
                            dom.el(self.ui.el_as("emu-badge-profile", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-profile", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-profile");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-network[data-section=network]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::USERS, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.network"));
                            });
                            dom.el(self.ui.el_as("emu-badge-network", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-network", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-network");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-mods[data-section=mods]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::BOX, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.mods"));
                            });
                            dom.el(self.ui.el_as("emu-badge-mods", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-mods", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-mods");
                        }
                        if dom.button(self.ui.el("button.emu-nav__tab#emu-tab-extras[data-section=extras]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::WRENCH, 2.0, true);
                            dom.el(el("span.emu-nav__label"), |dom| {
                                dom.text(self.t("emulator.tabs.extras"));
                            });
                            dom.el(self.ui.el_as("emu-badge-extras", "span.emu-nav__badge.hidden"), |dom| {
                                self.fill("emu-badge-extras", dom);
                            });
                        }).clicked() {
                            self.click("emu-tab-extras");
                        }
                    });
                    dom.el(el("div.emu-panels"), |dom| {
                        self.view_emu_panel_files(dom);
                        self.view_emu_panel_gamedata(dom);
                        self.view_emu_panel_controller(dom);
                        self.view_emu_panel_emulator(dom);
                        self.view_emu_panel_saves(dom);
                        self.view_emu_panel_profile(dom);
                        self.view_emu_panel_network(dom);
                        self.view_emu_panel_mods(dom);
                        self.view_emu_panel_extras(dom);
                    });
                });
                dom.el(self.ui.el("div.completion-message.hidden#emu-apply-status"), |dom| {
                    self.fill("emu-apply-status", dom);
                });
                dom.el(el("div.card__footer"), |dom| {
                    dom.el(el("div.card__footer-left"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-emu-start-over"), |dom| {
                            dom.text("↩");
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("emulator.startOver"));
                            });
                        }).clicked() {
                            self.click("btn-emu-start-over");
                        }
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-emu-new"), |dom| {
                            self.label("btn-emu-new", "emulator.goToHome", dom);
                        }).clicked() {
                            self.click("btn-emu-new");
                        }
                        if dom.button(self.ui.el("button.btn.btn--danger.hidden#btn-emu-revert"), |dom| {
                            self.label("btn-emu-revert", "emulator.revert", dom);
                        }).clicked() {
                            self.click("btn-emu-revert");
                        }
                    });
                    dom.el(el("div.card__footer-right"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline.hidden#btn-emu-save-settings"), |dom| {
                            self.label("btn-emu-save-settings", "emulator.saveSettings", dom);
                        }).clicked() {
                            self.click("btn-emu-save-settings");
                        }
                        if dom.button(self.ui.el("button.btn.btn--primary#btn-emu-apply"), |dom| {
                            self.label("btn-emu-apply", "emulator.apply", dom);
                        }).clicked() {
                            self.click("btn-emu-apply");
                        }
                    });
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_files(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel.is-active#emu-panel-files[data-section=files]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.detectedLabel"));
            });
            dom.el(self.ui.el("div.denuvo-note.hidden#emu-denuvo-note"), |dom| {
                dom.el(el("strong"), |dom| {
                    dom.text(self.t("select.denuvoTitle"));
                });
                dom.el(el("span"), |dom| {
                    dom.text(self.t("emulator.denuvoNote"));
                });
            });
            dom.el(self.ui.el("div.emu-drm-section.hidden#emu-drm-section"), |dom| {
                dom.el(el("div.emu-drm-header"), |dom| {
                    dom.icon_with(el("svg.emu-drm-icon").size(20.0, 20.0), icons::ALERT_TRIANGLE, 2.0, true);
                    dom.el(el("div.emu-drm-header__text"), |dom| {
                        dom.el(el("h3.emu-drm-title"), |dom| {
                            self.label("emu-drm-title", "emulator.drmTitle", dom);
                        });
                        dom.el(el("p.emu-drm-description"), |dom| {
                            self.label("emu-drm-description", "emulator.drmDescription", dom);
                        });
                    });
                });
                dom.el(self.ui.el("div.emu-drm-list#emu-drm-list"), |dom| {
                    self.fill("emu-drm-list", dom);
                });
                dom.el(self.ui.el("div.emu-drm-status-wrap.hidden#emu-drm-status-wrap"), |dom| {
                    if dom.button(self.ui.el("button.emu-drm-copy-btn#btn-emu-drm-copy").title_or(self.t("emulator.drmCopyLog")), |dom| {
                        if !self.override_content("btn-emu-drm-copy", dom) {
                            dom.icon_with(el("svg").size(14.0, 14.0), icons::COPY, 2.0, true);
                        }
                    }).clicked() {
                        self.click("btn-emu-drm-copy");
                    }
                    dom.el(self.ui.el("div.emu-drm-status#emu-drm-status"), |dom| {
                        self.fill("emu-drm-status", dom);
                    });
                });
                dom.el(el("div.emu-drm-actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--danger#btn-emu-drm-remove"), |dom| {
                        self.label("btn-emu-drm-remove", "emulator.drmRemove", dom);
                    }).clicked() {
                        self.click("btn-emu-drm-remove");
                    }
                    dom.el(el("span.emu-drm-attribution"), |dom| {
                        dom.text(self.t("emulator.drmAttribution"));
                    });
                });
            });
            dom.el(el("div.emu-section"), |dom| {
                dom.el(self.ui.el("div.emu-patch-version.hidden#emu-patch-version"), |dom| {
                    dom.el(self.ui.el("span#emu-patch-version-text"), |dom| {
                        self.fill("emu-patch-version-text", dom);
                    });
                    if dom.button(self.ui.el("button.btn.btn--small.btn--primary.hidden#btn-emu-update-gbe"), |dom| {
                        self.fill("btn-emu-update-gbe", dom);
                    }).clicked() {
                        self.click("btn-emu-update-gbe");
                    }
                });
                dom.el(self.ui.el("div.emu-file-list#emu-file-list"), |dom| {
                    self.fill("emu-file-list", dom);
                });
                dom.el(self.ui.el("div.emu-drm-restore.hidden#emu-drm-restore"), |dom| {
                    dom.el(el("p.emu-gamedata__key-label"), |dom| {
                        dom.text(self.t("emulator.drmRestore.title"));
                    });
                    dom.el(el("p.dd-path__hint"), |dom| {
                        dom.text(self.t("emulator.drmRestore.hint"));
                    });
                    dom.el(self.ui.el("div.emu-drm-restore__list#emu-drm-restore-list"), |dom| {
                        self.fill("emu-drm-restore-list", dom);
                    });
                });
                dom.el(self.ui.el("p.emu-file-empty.hidden#emu-file-empty"), |dom| {
                    self.label("emu-file-empty", "emulator.noFiles", dom);
                });
                dom.el(self.ui.el("p.emu-already-patched.hidden#emu-already-patched"), |dom| {
                    self.fill("emu-already-patched", dom);
                });
            });
            dom.el(self.ui.el("div.emu-section.hidden#emu-method-section"), |dom| {
                dom.el(el("label.settings-section__label"), |dom| {
                    dom.text(self.t("emulator.method.label"));
                });
                dom.el(el("div.emu-variant-group"), |dom| {
                    if dom.button(el("label.emu-variant").state(HAS_CHECKED, self.ui.checked("emu-method-replace")), |dom| {
                        dom.empty(self.ui.el_as("emu-method-replace", "input[type=radio]").state(CHECKED, self.ui.checked("emu-method-replace")));
                        dom.el(el("span.emu-variant__body"), |dom| {
                            dom.el(el("span.emu-variant__title"), |dom| {
                                dom.text(self.t("emulator.method.replace"));
                            });
                            dom.el(el("span.emu-variant__hint"), |dom| {
                                dom.text(self.t("emulator.method.replaceHint"));
                            });
                        });
                    }).clicked() {
                        self.radio("emu-method", "emu-method-replace");
                    }
                    if dom.button(el("label.emu-variant").state(HAS_CHECKED, self.ui.checked("emu-method-coldclient")), |dom| {
                        dom.empty(self.ui.el_as("emu-method-coldclient", "input[type=radio]").state(CHECKED, self.ui.checked("emu-method-coldclient")));
                        dom.el(el("span.emu-variant__body"), |dom| {
                            dom.el(el("span.emu-variant__title"), |dom| {
                                dom.text(self.t("emulator.method.coldclient"));
                            });
                            dom.el(el("span.emu-variant__hint"), |dom| {
                                dom.text(self.t("emulator.method.coldclientHint"));
                            });
                        });
                    }).clicked() {
                        self.radio("emu-method", "emu-method-coldclient");
                    }
                });
                dom.el(self.ui.el("div.emu-gamedata__key.hidden#emu-cold-options"), |dom| {
                    if dom.button(el("label.emu-gamedata__key-label#emu-cold-exe-label[for=emu-cold-exe]"), |dom| {
                        dom.text(self.t("emulator.method.exe"));
                    }).clicked() {
                        self.label_click("emu-cold-exe");
                    }
                    self.select(dom, self.ui.el("select.emu-field__input.emu-field__select#emu-cold-exe"), "emu-cold-exe", &[]);
                    dom.el(el("p.emu-bypass-row__hint"), |dom| {
                        dom.text(self.t("emulator.method.exeHint"));
                    });
                });
                dom.el(self.ui.el("div.emu-cold-steam.hidden#emu-cold-steam"), |dom| {
                    dom.el(el("p.dd-path__hint"), |dom| {
                        dom.text(self.t("emulator.method.steamRunning"));
                    });
                    if dom.button(self.ui.el("button.btn.btn--small.btn--primary#btn-emu-cold-steam"), |dom| {
                        self.label("btn-emu-cold-steam", "emulator.method.closeSteam", dom);
                    }).clicked() {
                        self.click("btn-emu-cold-steam");
                    }
                });
            });
            dom.el(self.ui.el("div.emu-section#emu-variant-section"), |dom| {
                dom.el(el("label.settings-section__label"), |dom| {
                    dom.text(self.t("emulator.variantLabel"));
                });
                dom.el(el("div.emu-variant-group"), |dom| {
                    if dom.button(el("label.emu-variant").state(HAS_CHECKED, self.ui.checked("emu-variant-regular")), |dom| {
                        dom.empty(self.ui.el_as("emu-variant-regular", "input[type=radio]").state(CHECKED, self.ui.checked("emu-variant-regular")));
                        dom.el(el("span.emu-variant__body"), |dom| {
                            dom.el(el("span.emu-variant__title"), |dom| {
                                dom.text(self.t("emulator.variantRegular"));
                            });
                            dom.el(el("span.emu-variant__hint"), |dom| {
                                dom.text(self.t("emulator.variantRegularHint"));
                            });
                        });
                    }).clicked() {
                        self.radio("emu-variant", "emu-variant-regular");
                    }
                    if dom.button(el("label.emu-variant").state(HAS_CHECKED, self.ui.checked("emu-variant-experimental")), |dom| {
                        dom.empty(self.ui.el_as("emu-variant-experimental", "input[type=radio]").state(CHECKED, self.ui.checked("emu-variant-experimental")));
                        dom.el(el("span.emu-variant__body"), |dom| {
                            dom.el(el("span.emu-variant__title"), |dom| {
                                dom.text(self.t("emulator.variantExperimental"));
                            });
                            dom.el(el("span.emu-variant__hint"), |dom| {
                                dom.text(self.t("emulator.variantExperimentalHint"));
                            });
                        });
                    }).clicked() {
                        self.radio("emu-variant", "emu-variant-experimental");
                    }
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_gamedata(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-gamedata[data-section=gamedata]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.tabs.gameData"));
            });
            dom.el(self.ui.el("div.emu-section.emu-gamedata-section#emu-gamedata-section"), |dom| {
                dom.el(el("div.emu-bypass-row"), |dom| {
                    dom.el(el("div.emu-bypass-row__text"), |dom| {
                        dom.el(el("span.emu-bypass-row__label"), |dom| {
                            dom.text(self.t("emulator.gameData.label"));
                        });
                        dom.el(el("p.emu-bypass-row__hint"), |dom| {
                            self.html_key("emulator.gameData.hint", dom);
                        });
                    });
                    if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("emu-gamedata-toggle")), |dom| {
                        dom.empty(self.ui.el("input#emu-gamedata-toggle[type=checkbox]").state(CHECKED, self.ui.checked("emu-gamedata-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    }).clicked() {
                        self.toggle("emu-gamedata-toggle");
                    }
                });
                dom.el(el("div.emu-gamedata__key"), |dom| {
                    if dom.button(el("label.emu-gamedata__key-label[for=emu-webapi-key]"), |dom| {
                        dom.text(self.t("settings.steamWebApiKey"));
                    }).clicked() {
                        self.label_click("emu-webapi-key");
                    }
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#emu-webapi-key[type=password]");
                            let placeholder = self.ui.attr("emu-webapi-key", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.steamWebApiKeyPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("emu-webapi-key"), &placeholder, true);
                            self.input_events("emu-webapi-key", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-webapi-save"), |dom| {
                            self.label("btn-emu-webapi-save", "emulator.gameData.saveKey", dom);
                        }).clicked() {
                            self.click("btn-emu-webapi-save");
                        }
                    });
                    dom.el(el("p.emu-bypass-row__hint"), |dom| {
                        self.html_key("settings.steamWebApiKeyHint", dom);
                    });
                });
                dom.el(el("div.emu-gamedata__media"), |dom| {
                    dom.el(self.ui.el("span.emu-gamedata__key-label#emu-gamedata-media-label"), |dom| {
                        self.label("emu-gamedata-media-label", "emulator.gameData.media", dom);
                    });
                    dom.el(self.ui.el("div.segmented#emu-gamedata-media"), |dom| {
                        if dom.button(self.ui.el_as("emu-gamedata-media-off", "button.segmented__option.is-active"), |dom| {
                            dom.text(self.t("emulator.gameData.mediaOff"));
                        }).clicked() {
                            self.click("emu-gamedata-media:off");
                        }
                        if dom.button(self.ui.el_as("emu-gamedata-media-images", "button.segmented__option"), |dom| {
                            dom.text(self.t("emulator.gameData.mediaImages"));
                        }).clicked() {
                            self.click("emu-gamedata-media:images");
                        }
                        if dom.button(self.ui.el_as("emu-gamedata-media-all", "button.segmented__option"), |dom| {
                            dom.text(self.t("emulator.gameData.mediaAll"));
                        }).clicked() {
                            self.click("emu-gamedata-media:all");
                        }
                    });
                    dom.el(el("p.emu-bypass-row__hint"), |dom| {
                        self.html_key("emulator.gameData.mediaHint", dom);
                    });
                });
                dom.el(el("div.emu-gamedata__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-gamedata"), |dom| {
                        self.label("btn-emu-gamedata", "emulator.gameData.generate", dom);
                    }).clicked() {
                        self.click("btn-emu-gamedata");
                    }
                    dom.el(self.ui.el("p.dd-path__hint.hidden#emu-gamedata-status"), |dom| {
                        self.fill("emu-gamedata-status", dom);
                    });
                });
            });
            dom.el(el("p.dd-path__hint.emu-achievement-hint"), |dom| {
                self.html_key("emulator.achievementWatcherHint", dom);
            });
        });
    }

    pub(crate) fn view_emu_panel_controller(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-controller[data-section=controller]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.controller.title"));
            });
            dom.el(self.ui.el("div.emu-section.emu-gamedata-section#emu-controller-section"), |dom| {
                dom.el(el("div.emu-bypass-row"), |dom| {
                    dom.el(el("div.emu-bypass-row__text"), |dom| {
                        dom.el(el("span.emu-bypass-row__label"), |dom| {
                            dom.text(self.t("emulator.controller.label"));
                        });
                        dom.el(el("p.emu-bypass-row__hint"), |dom| {
                            self.html_key("emulator.controller.hint", dom);
                        });
                    });
                    if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("emu-controller-toggle")), |dom| {
                        dom.empty(self.ui.el("input#emu-controller-toggle[type=checkbox]").state(CHECKED, self.ui.checked("emu-controller-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    }).clicked() {
                        self.toggle("emu-controller-toggle");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint#emu-controller-status"), |dom| {
                    self.fill("emu-controller-status", dom);
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-controller-build"), |dom| {
                    self.fill("emu-controller-build", dom);
                });
                dom.el(el("div.emu-gamedata__key"), |dom| {
                    dom.el(el("span.emu-gamedata__key-label"), |dom| {
                        dom.text(self.t("emulator.controller.layoutFile"));
                    });
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#emu-controller-file[readonly][type=text]");
                            let placeholder = self.ui.attr("emu-controller-file", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("emulator.controller.noFile").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("emu-controller-file"), &placeholder, false);
                            self.input_events("emu-controller-file", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-controller-file"), |dom| {
                            self.label("btn-emu-controller-file", "emulator.controller.chooseFile", dom);
                        }).clicked() {
                            self.click("btn-emu-controller-file");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.hidden#btn-emu-controller-clear"), |dom| {
                            self.label("btn-emu-controller-clear", "emulator.controller.clearFile", dom);
                        }).clicked() {
                            self.click("btn-emu-controller-clear");
                        }
                    });
                    dom.el(el("p.emu-bypass-row__hint"), |dom| {
                        self.html_key("emulator.controller.layoutFileHint", dom);
                    });
                });
                dom.el(el("div.emu-gamedata__key"), |dom| {
                    if dom.button(el("label.emu-gamedata__key-label#emu-controller-type-label[for=emu-controller-type]"), |dom| {
                        dom.text(self.t("emulator.fields.controllerType"));
                    }).clicked() {
                        self.label_click("emu-controller-type");
                    }
                    self.select(dom, self.ui.el("select.emu-field__input.emu-field__select#emu-controller-type"), "emu-controller-type", &[("", "emulator.fields.controllerDefault", "gbe_fork default (Xbox 360)"), ("XBOX360", "", "Xbox 360"), ("XBOXONE", "", "Xbox One"), ("PS3", "", "PlayStation 3"), ("PS4", "", "PlayStation 4"), ("PS5", "", "PlayStation 5"), ("SWITCH", "", "Nintendo Switch")]);
                    dom.el(el("p.emu-bypass-row__hint"), |dom| {
                        dom.text(self.t("emulator.fields.controllerTypeHint"));
                    });
                });
                dom.el(el("p.dd-path__hint.emu-controller__xinput"), |dom| {
                    dom.text(self.t("emulator.controller.xinputHint"));
                });
                dom.el(el("div.emu-field.emu-field--toggle"), |dom| {
                    dom.el(el("div.emu-field__label-block"), |dom| {
                        dom.el(el("span.emu-field__label"), |dom| {
                            dom.text(self.t("emulator.controller.steamInput"));
                        });
                        dom.el(el("p.emu-field__hint"), |dom| {
                            dom.text(self.t("emulator.controller.steamInputHint"));
                        });
                    });
                    if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("emu-set-disable_steamoverlaygameid_env_var")), |dom| {
                        dom.empty(self.ui.el_as("emu-set-disable_steamoverlaygameid_env_var", "input[type=checkbox]").state(CHECKED, self.ui.checked("emu-set-disable_steamoverlaygameid_env_var")));
                        dom.empty(el("span.toggle-switch__slider"));
                    }).clicked() {
                        self.toggle("emu-set-disable_steamoverlaygameid_env_var");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-controller-result"), |dom| {
                    self.fill("emu-controller-result", dom);
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_emulator(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-emulator[data-section=emulator]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.tabs.emulatorTitle"));
            });
            dom.el(el("div.emu-section"), |dom| {
                dom.el(self.ui.el("p.emu-settings-hint#emu-settings-hint"), |dom| {
                    self.label("emu-settings-hint", "emulator.settingsHint", dom);
                });
                dom.el(el("div.emu-settings"), |dom| {
                    dom.el(el("div.emu-settings__bar"), |dom| {
                        {
                            let element = self.ui.el("input.emu-field__input.emu-settings__search#emu-settings-search[type=search]");
                            let placeholder = self.ui.attr("emu-settings-search", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("emulator.settingsSearch").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("emu-settings-search"), &placeholder, false);
                            self.input_events("emu-settings-search", &response);
                        }
                        if dom.button(el("label.emu-settings__only").state(HAS_CHECKED, self.ui.checked("emu-settings-changed")), |dom| {
                            dom.empty(self.ui.el("input#emu-settings-changed[type=checkbox]").state(CHECKED, self.ui.checked("emu-settings-changed")));
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("emulator.settingsOnlyChanged"));
                            });
                        }).clicked() {
                            self.toggle("emu-settings-changed");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-settings-reset"), |dom| {
                            self.label("btn-emu-settings-reset", "emulator.settingsReset", dom);
                        }).clicked() {
                            self.click("btn-emu-settings-reset");
                        }
                    });
                    dom.el(self.ui.el("div.emu-settings__groups#emu-settings-groups"), |dom| {
                        self.fill("emu-settings-groups", dom);
                    });
                    dom.el(self.ui.el("div.emu-settings__fields#emu-settings-fields"), |dom| {
                        self.fill("emu-settings-fields", dom);
                    });
                    dom.el(self.ui.el("p.dd-path__hint.hidden#emu-settings-empty"), |dom| {
                        self.label("emu-settings-empty", "emulator.settingsNoMatch", dom);
                    });
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_saves(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-saves[data-section=saves]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.saves.title"));
            });
            dom.el(self.ui.el_as("emu-needs-saves", "p.emu-extra__needs.hidden"), |dom| {
                dom.text(self.t("emulator.extras.needsTarget"));
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-saves")), |dom| {
                dom.el(el("span.emu-gamedata__key-label"), |dom| {
                    dom.text(self.t("emulator.saves.folder"));
                });
                dom.el(self.ui.el("div.emu-extra-list#emu-saves-list"), |dom| {
                    self.fill("emu-saves-list", dom);
                });
                dom.el(el("p.emu-bypass-row__hint"), |dom| {
                    self.html_key("emulator.saves.hint", dom);
                });
                dom.el(el("div.emu-gamedata__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-saves-open"), |dom| {
                        self.label("btn-emu-saves-open", "emulator.saves.open", dom);
                    }).clicked() {
                        self.click("btn-emu-saves-open");
                    }
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-saves-backup"), |dom| {
                        self.label("btn-emu-saves-backup", "emulator.saves.backup", dom);
                    }).clicked() {
                        self.click("btn-emu-saves-backup");
                    }
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-saves-restore"), |dom| {
                        self.label("btn-emu-saves-restore", "emulator.saves.restore", dom);
                    }).clicked() {
                        self.click("btn-emu-saves-restore");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-saves-status"), |dom| {
                    self.fill("emu-saves-status", dom);
                });
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-saves")), |dom| {
                dom.el(el("div.emu-extra-head"), |dom| {
                    dom.el(el("span.emu-gamedata__key-label"), |dom| {
                        dom.text(self.t("emulator.achievements.title"));
                    });
                    dom.el(self.ui.el("span.dd-path__hint#emu-ach-count"), |dom| {
                        self.fill("emu-ach-count", dom);
                    });
                });
                dom.el(self.ui.el("div.emu-ach-list#emu-ach-list"), |dom| {
                    self.fill("emu-ach-list", dom);
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-ach-empty"), |dom| {
                    self.fill("emu-ach-empty", dom);
                });
                dom.el(el("div.emu-gamedata__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-ach-reset"), |dom| {
                        self.label("btn-emu-ach-reset", "emulator.achievements.resetAll", dom);
                    }).clicked() {
                        self.click("btn-emu-ach-reset");
                    }
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-stats-reset"), |dom| {
                        self.label("btn-emu-stats-reset", "emulator.achievements.resetStats", dom);
                    }).clicked() {
                        self.click("btn-emu-stats-reset");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-ach-status"), |dom| {
                    self.fill("emu-ach-status", dom);
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_profile(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-profile[data-section=profile]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.profile.title"));
            });
            dom.el(self.ui.el_as("emu-needs-profile", "p.emu-extra__needs.hidden"), |dom| {
                dom.text(self.t("emulator.extras.needsTarget"));
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-profile")), |dom| {
                dom.el(el("span.emu-gamedata__key-label"), |dom| {
                    dom.text(self.t("emulator.profile.avatar"));
                });
                dom.el(el("div.emu-avatar"), |dom| {
                    dom.el(self.ui.el("div.emu-avatar__img#emu-avatar-preview"), |dom| {
                        self.fill("emu-avatar-preview", dom);
                    });
                    dom.el(el("div.emu-avatar__side"), |dom| {
                        dom.el(el("p.emu-bypass-row__hint"), |dom| {
                            dom.text(self.t("emulator.profile.avatarHint"));
                        });
                        dom.el(el("div.emu-gamedata__actions"), |dom| {
                            if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-avatar"), |dom| {
                                self.label("btn-emu-avatar", "emulator.profile.choose", dom);
                            }).clicked() {
                                self.click("btn-emu-avatar");
                            }
                            if dom.button(self.ui.el("button.btn.btn--small.btn--outline.hidden#btn-emu-avatar-clear"), |dom| {
                                self.label("btn-emu-avatar-clear", "emulator.profile.remove", dom);
                            }).clicked() {
                                self.click("btn-emu-avatar-clear");
                            }
                        });
                    });
                });
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-profile")), |dom| {
                dom.el(el("span.emu-gamedata__key-label"), |dom| {
                    dom.text(self.t("emulator.profile.overlayAssets"));
                });
                dom.el(el("p.emu-bypass-row__hint"), |dom| {
                    dom.text(self.t("emulator.profile.overlayHint"));
                });
                dom.el(el("div.emu-asset-row"), |dom| {
                    dom.el(el("div.emu-asset-row__text"), |dom| {
                        dom.el(el("span.emu-field__label"), |dom| {
                            dom.text(self.t("emulator.profile.font"));
                        });
                        dom.el(el("span.emu-asset-row__value.dd-path__hint"), |dom| {
                            self.label("emu-asset-value-font", "emulator.profile.defaultFont", dom);
                        });
                    });
                    dom.el(el("div.emu-font-system"), |dom| {
                        if dom.button(el("label.sr-only#emu-font-system-label[for=emu-font-system]"), |dom| {
                            dom.text(self.t("emulator.profile.systemFonts"));
                        }).clicked() {
                            self.label_click("emu-font-system");
                        }
                        self.select(dom, self.ui.el("select#emu-font-system"), "emu-font-system", &[("", "emulator.profile.systemFontsLoading", "Loading installed fonts…")]);
                    });
                    if dom.button(el("button.btn.btn--small.btn--outline"), |dom| {
                        dom.text(self.t("emulator.profile.fontFile"));
                    }).clicked() {
                        self.click("asset-pick:font");
                    }
                    if dom.button(self.ui.el_as("emu-asset-clear-font", "button.btn.btn--small.btn--outline.hidden"), |dom| {
                        dom.text(self.t("emulator.profile.remove"));
                    }).clicked() {
                        self.click("asset-clear:font");
                    }
                });
                dom.el(el("div.emu-asset-row"), |dom| {
                    dom.el(el("div.emu-asset-row__text"), |dom| {
                        dom.el(el("span.emu-field__label"), |dom| {
                            dom.text(self.t("emulator.profile.achievementSound"));
                        });
                        dom.el(el("span.emu-asset-row__value.dd-path__hint"), |dom| {
                            self.label("emu-asset-value-achievement", "emulator.profile.noSound", dom);
                        });
                    });
                    if dom.button(el("button.btn.btn--small.btn--outline"), |dom| {
                        dom.text(self.t("emulator.profile.choose"));
                    }).clicked() {
                        self.click("asset-pick:achievement");
                    }
                    if dom.button(self.ui.el_as("emu-asset-clear-achievement", "button.btn.btn--small.btn--outline.hidden"), |dom| {
                        dom.text(self.t("emulator.profile.remove"));
                    }).clicked() {
                        self.click("asset-clear:achievement");
                    }
                });
                dom.el(el("div.emu-asset-row"), |dom| {
                    dom.el(el("div.emu-asset-row__text"), |dom| {
                        dom.el(el("span.emu-field__label"), |dom| {
                            dom.text(self.t("emulator.profile.friendSound"));
                        });
                        dom.el(el("span.emu-asset-row__value.dd-path__hint"), |dom| {
                            self.label("emu-asset-value-friend", "emulator.profile.noSound", dom);
                        });
                    });
                    if dom.button(el("button.btn.btn--small.btn--outline"), |dom| {
                        dom.text(self.t("emulator.profile.choose"));
                    }).clicked() {
                        self.click("asset-pick:friend");
                    }
                    if dom.button(self.ui.el_as("emu-asset-clear-friend", "button.btn.btn--small.btn--outline.hidden"), |dom| {
                        dom.text(self.t("emulator.profile.remove"));
                    }).clicked() {
                        self.click("asset-clear:friend");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-profile-status"), |dom| {
                    self.fill("emu-profile-status", dom);
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_network(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-network[data-section=network]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.network.title"));
            });
            dom.el(self.ui.el_as("emu-needs-network", "p.emu-extra__needs.hidden"), |dom| {
                dom.text(self.t("emulator.extras.needsTarget"));
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-network")), |dom| {
                if dom.button(el("label.emu-gamedata__key-label[for=emu-broadcasts]"), |dom| {
                    dom.text(self.t("emulator.network.broadcasts"));
                }).clicked() {
                    self.label_click("emu-broadcasts");
                }
                {
                    let element = self.ui.el("textarea.emu-field__input.emu-broadcasts#emu-broadcasts");
                    let placeholder = self.t("emulator.network.broadcastsPlaceholder").to_string();
                    let response = dom.textarea(element, self.ui.value_mut("emu-broadcasts"), &placeholder, 4);
                    self.input_events("emu-broadcasts", &response);
                }
                dom.el(el("p.emu-bypass-row__hint"), |dom| {
                    self.html_key("emulator.network.broadcastsHint", dom);
                });
                dom.el(el("div.emu-gamedata__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-broadcasts"), |dom| {
                        self.label("btn-emu-broadcasts", "emulator.network.save", dom);
                    }).clicked() {
                        self.click("btn-emu-broadcasts");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-broadcasts-status"), |dom| {
                    self.fill("emu-broadcasts-status", dom);
                });
            });
            dom.el(self.ui.el("p.dd-path__hint.hidden#emu-lobby-later"), |dom| {
                self.label("emu-lobby-later", "emulator.network.lobbyLater", dom);
            });
            dom.el(self.ui.el("div.emu-section.emu-gamedata-section#emu-lobby-section"), |dom| {
                dom.el(el("span.emu-gamedata__key-label"), |dom| {
                    dom.text(self.t("emulator.network.lobbyTitle"));
                });
                dom.el(el("p.emu-bypass-row__hint"), |dom| {
                    self.html_key("emulator.network.lobbyHint", dom);
                });
                dom.el(el("div.emu-gamedata__actions"), |dom| {
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-lobby"), |dom| {
                        self.label("btn-emu-lobby", "emulator.network.lobbyStart", dom);
                    }).clicked() {
                        self.click("btn-emu-lobby");
                    }
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-lobby-status"), |dom| {
                    self.fill("emu-lobby-status", dom);
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_mods(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-mods[data-section=mods]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.mods.title"));
            });
            dom.el(self.ui.el_as("emu-needs-mods", "p.emu-extra__needs.hidden"), |dom| {
                dom.text(self.t("emulator.extras.needsTarget"));
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-mods")), |dom| {
                dom.el(self.ui.el("div.emu-extra-list#emu-mods-list"), |dom| {
                    self.fill("emu-mods-list", dom);
                });
                dom.el(self.ui.el("p.dd-path__hint#emu-mods-empty"), |dom| {
                    self.label("emu-mods-empty", "emulator.mods.empty", dom);
                });
            });
            dom.el(el("div.emu-section.emu-gamedata-section").class_if("hidden", self.ui.flag("emu-extra-hide-mods")), |dom| {
                if dom.button(el("label.emu-gamedata__key-label[for=emu-mod-id]"), |dom| {
                    dom.text(self.t("emulator.mods.add"));
                }).clicked() {
                    self.label_click("emu-mod-id");
                }
                dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                    {
                        let element = self.ui.el("input.dd-path__input#emu-mod-id[type=text]");
                        let placeholder = self.ui.attr("emu-mod-id", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("emulator.mods.idPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("emu-mod-id"), &placeholder, false);
                        self.input_events("emu-mod-id", &response);
                    }
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-mod-folder"), |dom| {
                        self.label("btn-emu-mod-folder", "emulator.mods.chooseFolder", dom);
                    }).clicked() {
                        self.click("btn-emu-mod-folder");
                    }
                    if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-emu-mod-file"), |dom| {
                        self.label("btn-emu-mod-file", "emulator.mods.chooseFile", dom);
                    }).clicked() {
                        self.click("btn-emu-mod-file");
                    }
                });
                dom.el(el("p.emu-bypass-row__hint"), |dom| {
                    dom.text(self.t("emulator.mods.hint"));
                });
                dom.el(self.ui.el("p.dd-path__hint.hidden#emu-mods-status"), |dom| {
                    self.fill("emu-mods-status", dom);
                });
            });
        });
    }

    pub(crate) fn view_emu_panel_extras(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("section.emu-panel#emu-panel-extras[data-section=extras]"), |dom| {
            dom.el(el("h3.emu-panel__title"), |dom| {
                dom.text(self.t("emulator.tabs.extras"));
            });
            dom.el(self.ui.el("div.emu-section.emu-bypass-section#emu-bypass-section"), |dom| {
                dom.el(el("div.emu-bypass-row"), |dom| {
                    dom.el(el("div.emu-bypass-row__text"), |dom| {
                        dom.el(el("span.emu-bypass-row__label"), |dom| {
                            dom.text(self.t("emulator.bypassLabel"));
                        });
                        dom.el(el("p.emu-bypass-row__hint"), |dom| {
                            self.label_html("emu-bypass-hint", "emulator.bypassHint", dom);
                        });
                    });
                    if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("emu-bypass-toggle")), |dom| {
                        dom.empty(self.ui.el("input#emu-bypass-toggle[type=checkbox]").state(CHECKED, self.ui.checked("emu-bypass-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    }).clicked() {
                        self.toggle("emu-bypass-toggle");
                    }
                });
            });
            dom.el(self.ui.el("div.emu-section.emu-dlc-merge-section.hidden#emu-dlc-merge-section"), |dom| {
                dom.el(el("div.emu-bypass-row"), |dom| {
                    dom.el(el("div.emu-bypass-row__text"), |dom| {
                        dom.el(el("span.emu-bypass-row__label"), |dom| {
                            dom.text(self.t("emulator.dlcMergeLabel"));
                        });
                        dom.el(self.ui.el("p.emu-bypass-row__hint#emu-dlc-merge-hint"), |dom| {
                            self.label("emu-dlc-merge-hint", "emulator.dlcMergeHint", dom);
                        });
                        dom.el(self.ui.el("p.dd-path__hint.hidden#emu-dlc-merge-status"), |dom| {
                            self.fill("emu-dlc-merge-status", dom);
                        });
                    });
                    if dom.button(self.ui.el("button.btn.btn--primary.btn--small#btn-emu-merge-dlcs"), |dom| {
                        self.label("btn-emu-merge-dlcs", "emulator.dlcMergeButton", dom);
                    }).clicked() {
                        self.click("btn-emu-merge-dlcs");
                    }
                });
            });
        });
    }
}
