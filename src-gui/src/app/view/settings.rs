#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_settings_modal(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.modal.hidden#settings-modal"), |dom| {
            if dom.button(el("div.modal__backdrop"), |dom| {
                let _ = &dom;
            }).clicked() {
                self.click("settings-modal>modal__backdrop");
            }
            dom.hit(el("div.modal__content.settings-modal-content.settings"), egui::Sense::CLICK, |dom| {
                dom.el(el("header.settings__header"), |dom| {
                    dom.el(self.ui.el("h3.modal__title.settings__title#settings-modal-title"), |dom| {
                        dom.icon_with(el("svg.modal__title-icon").size(20.0, 20.0), icons::SETTINGS, 2.0, true);
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("settings.title"));
                        });
                    });
                    dom.el(el("label.settings__search"), |dom| {
                        dom.icon_with(el("svg").size(16.0, 16.0), icons::SEARCH, 2.0, true);
                        {
                            let element = self.ui.el("input#settings-search[type=search]");
                            let placeholder = self.ui.attr("settings-search", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.searchPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("settings-search"), &placeholder, false);
                            self.input_events("settings-search", &response);
                        }
                    });
                });
                dom.el(el("div.settings__body"), |dom| {
                    dom.el(el("nav.settings__nav"), |dom| {
                        if dom.button(self.ui.el_as("settings-tab-general", "button.settings__tab.is-active[data-section=general]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::SLIDERS, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("settings.sectionGeneral"));
                            });
                        }).clicked() {
                            self.click("settings-tab:general");
                        }
                        if dom.button(self.ui.el_as("settings-tab-downloads", "button.settings__tab[data-section=downloads]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::DOWNLOAD, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("settings.sectionDownloads"));
                            });
                        }).clicked() {
                            self.click("settings-tab:downloads");
                        }
                        if dom.button(self.ui.el_as("settings-tab-depots", "button.settings__tab[data-section=depots]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::BOX, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("settings.sectionDepots"));
                            });
                        }).clicked() {
                            self.click("settings-tab:depots");
                        }
                        if dom.button(self.ui.el_as("settings-tab-sources", "button.settings__tab[data-section=sources]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::GLOBE, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("settings.sectionSources"));
                            });
                        }).clicked() {
                            self.click("settings-tab:sources");
                        }
                        if dom.button(self.ui.el_as("settings-tab-steam", "button.settings__tab[data-section=steam]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::GAMEPAD, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("settings.sectionSteam"));
                            });
                        }).clicked() {
                            self.click("settings-tab:steam");
                        }
                        if dom.button(self.ui.el_as("settings-tab-privacy", "button.settings__tab[data-section=privacy]"), |dom| {
                            dom.icon_with(el("svg").size(18.0, 18.0), icons::SHIELD, 2.0, true);
                            dom.el(el("span"), |dom| {
                                dom.text(self.t("settings.sectionPrivacy"));
                            });
                        }).clicked() {
                            self.click("settings-tab:privacy");
                        }
                    });
                    dom.el(el("div.settings__panels"), |dom| {
                        dom.el(self.ui.el("p.settings__empty.hidden#settings-no-results"), |dom| {
                            self.label("settings-no-results", "settings.noResults", dom);
                        });
                        self.view_settings_panel_general(dom);
                        self.view_settings_panel_downloads(dom);
                        self.view_settings_panel_depots(dom);
                        self.view_settings_panel_sources(dom);
                        self.view_settings_panel_steam(dom);
                        self.view_settings_panel_privacy(dom);
                    });
                });
                dom.el(el("footer.modal__actions.settings__footer"), |dom| {
                    dom.el(self.ui.el("span.settings__dirty.hidden#settings-dirty"), |dom| {
                        self.label("settings-dirty", "settings.unsaved", dom);
                    });
                    if dom.button(self.ui.el("button.btn.btn--outline#btn-settings-cancel"), |dom| {
                        self.label("btn-settings-cancel", "settings.cancel", dom);
                    }).clicked() {
                        self.click("btn-settings-cancel");
                    }
                    if dom.button(self.ui.el("button.btn.btn--primary#btn-settings-save"), |dom| {
                        self.label("btn-settings-save", "settings.save", dom);
                    }).clicked() {
                        self.click("btn-settings-save");
                    }
                });
            });
        });
    }

    pub(crate) fn view_settings_panel_general(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el_as("settings-panel-general", "section.settings__panel.is-active[data-section=general]"), |dom| {
            dom.el(el("h4.settings__panel-title"), |dom| {
                dom.text(self.t("settings.sectionGeneral"));
            });
            dom.el(self.ui.el_as("settings-group-2", "div.settings-group"), |dom| {
                dom.el(self.ui.el_as("settings-row-3", "div.settings-row.settings-row--stack"), |dom| {
                    dom.el(el("div.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.language"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.languageHint"));
                        });
                    });
                    dom.el(self.ui.el("div.language-cards#settings-language-cards"), |dom| {
                        self.fill("settings-language-cards", dom);
                    });
                });
                if dom.button(self.ui.el_as("settings-row-4", "label.settings-row[for=auto-update-toggle]").state(HAS_CHECKED, self.ui.checked("auto-update-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.autoUpdate"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.autoUpdateHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#auto-update-toggle[type=checkbox]").state(CHECKED, self.ui.checked("auto-update-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("auto-update-toggle");
                }
                if dom.button(self.ui.el_as("settings-row-5", "label.settings-row[for=notification-sound-toggle]").state(HAS_CHECKED, self.ui.checked("notification-sound-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.notificationSound"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.notificationSoundHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#notification-sound-toggle[type=checkbox]").state(CHECKED, self.ui.checked("notification-sound-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("notification-sound-toggle");
                }
            });
        });
    }

    pub(crate) fn view_settings_panel_downloads(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el_as("settings-panel-downloads", "section.settings__panel[data-section=downloads]"), |dom| {
            dom.el(el("h4.settings__panel-title"), |dom| {
                dom.text(self.t("settings.sectionDownloads"));
            });
            dom.el(self.ui.el_as("settings-group-3", "div.settings-group"), |dom| {
                if dom.button(self.ui.el_as("settings-row-6", "label.settings-row[for=native-downloader-toggle]").state(HAS_CHECKED, self.ui.checked("native-downloader-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.nativeDownloaderToggle"));
                        });
                        dom.el(self.ui.el("span.settings-row__hint#native-downloader-hint"), |dom| {
                            self.label("native-downloader-hint", "settings.nativeDownloaderHint", dom);
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#native-downloader-toggle[type=checkbox]").state(CHECKED, self.ui.checked("native-downloader-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("native-downloader-toggle");
                }
                dom.el(self.ui.el_as("settings-row-7", "div.settings-row.settings-row--wrap"), |dom| {
                    if dom.button(el("label.settings-row__text[for=speed-limit-toggle]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.speedLimit"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.speedLimitHint"));
                        });
                    }).clicked() {
                        self.label_click("speed-limit-toggle");
                    }
                    if dom.button(el("label.toggle-switch").state(HAS_CHECKED, self.ui.checked("speed-limit-toggle")), |dom| {
                        dom.empty(self.ui.el("input#speed-limit-toggle[type=checkbox]").state(CHECKED, self.ui.checked("speed-limit-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    }).clicked() {
                        self.toggle("speed-limit-toggle");
                    }
                    dom.el(self.ui.el("div.speed-limit.settings-row__extra.hidden#speed-limit-controls"), |dom| {
                        dom.el(el("div.speed-limit__field"), |dom| {
                            {
                                let element = self.ui.el("input.speed-limit__input#speed-limit-input[type=number]");
                                let placeholder = self.ui.attr("speed-limit-input", "placeholder").map(str::to_string).unwrap_or_else(|| String::from(""));
                                let response = dom.input_with(element, self.ui.value_mut("speed-limit-input"), &placeholder, false);
                                self.input_events("speed-limit-input", &response);
                            }
                            dom.el(el("div.speed-limit__units"), |dom| {
                                if dom.button(self.ui.el_as("speed-unit-mb", "button.speed-limit__unit.is-active"), |dom| {
                                    dom.text("MB/s");
                                }).clicked() {
                                    self.click("speed-unit:MB/s");
                                }
                                if dom.button(self.ui.el_as("speed-unit-mbit", "button.speed-limit__unit"), |dom| {
                                    dom.text("Mbit/s");
                                }).clicked() {
                                    self.click("speed-unit:Mbit/s");
                                }
                            });
                        });
                    });
                });
                dom.el(self.ui.el_as("settings-row-8", "div.settings-row.settings-row--wrap"), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.folderStyle"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.folderStyleHint"));
                        });
                    });
                    dom.el(self.ui.el("div.segmented#folder-style-options"), |dom| {
                        if dom.button(self.ui.el_as("folder-style-options-", "button.segmented__option.is-active"), |dom| {
                            dom.text("220 - Half-Life 2");
                        }).clicked() {
                            self.click("folder-style-options:");
                        }
                        if dom.button(self.ui.el_as("folder-style-options-underscore", "button.segmented__option"), |dom| {
                            dom.text("220_Half-Life_2");
                        }).clicked() {
                            self.click("folder-style-options:underscore");
                        }
                        if dom.button(self.ui.el_as("folder-style-options-dash", "button.segmented__option"), |dom| {
                            dom.text("220-Half-Life-2");
                        }).clicked() {
                            self.click("folder-style-options:dash");
                        }
                    });
                });
                if dom.button(self.ui.el_as("settings-row-9", "label.settings-row[for=cancel-keep-files-toggle]").state(HAS_CHECKED, self.ui.checked("cancel-keep-files-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.cancelKeepFilesToggle"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.cancelKeepFilesHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#cancel-keep-files-toggle[type=checkbox]").state(CHECKED, self.ui.checked("cancel-keep-files-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("cancel-keep-files-toggle");
                }
            });
            dom.el(self.ui.el_as("settings-title-1", "h5.settings__group-title"), |dom| {
                dom.text(self.t("settings.groupFineTuning"));
            });
            dom.el(self.ui.el_as("settings-group-4", "div.settings-group"), |dom| {
                if dom.button(self.ui.el_as("settings-row-10", "label.settings-row[for=max-retries-input]"), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.maxRetries"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.maxRetriesHint"));
                        });
                    });
                    {
                        let element = self.ui.el("input.dd-path__input.settings-row__number#max-retries-input[type=number]");
                        let placeholder = self.ui.attr("max-retries-input", "placeholder").map(str::to_string).unwrap_or_else(|| String::from(""));
                        let response = dom.input_with(element, self.ui.value_mut("max-retries-input"), &placeholder, false);
                        self.input_events("max-retries-input", &response);
                    }
                }).clicked() {
                    self.label_click("max-retries-input");
                }
                if dom.button(self.ui.el_as("settings-row-11", "label.settings-row[for=chunk-concurrency-input]"), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.chunkConcurrency"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.chunkConcurrencyHint"));
                        });
                    });
                    {
                        let element = self.ui.el("input.dd-path__input.settings-row__number#chunk-concurrency-input[type=number]");
                        let placeholder = self.ui.attr("chunk-concurrency-input", "placeholder").map(str::to_string).unwrap_or_else(|| String::from(""));
                        let response = dom.input_with(element, self.ui.value_mut("chunk-concurrency-input"), &placeholder, false);
                        self.input_events("chunk-concurrency-input", &response);
                    }
                }).clicked() {
                    self.label_click("chunk-concurrency-input");
                }
                dom.el(self.ui.el("div.settings-row.settings-row--stack#dd-extra-args-row"), |dom| {
                    if dom.button(el("label.settings-row__text[for=dd-extra-args-input]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.ddExtraArgs"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.ddExtraArgsHint"));
                        });
                    }).clicked() {
                        self.label_click("dd-extra-args-input");
                    }
                    {
                        let element = self.ui.el("input.dd-path__input#dd-extra-args-input[type=text]");
                        let placeholder = self.ui.attr("dd-extra-args-input", "placeholder").map(str::to_string).unwrap_or_else(|| String::from("-max-downloads 8 -verify-all"));
                        let response = dom.input_with(element, self.ui.value_mut("dd-extra-args-input"), &placeholder, false);
                        self.input_events("dd-extra-args-input", &response);
                    }
                });
            });
        });
    }

    pub(crate) fn view_settings_panel_depots(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el_as("settings-panel-depots", "section.settings__panel[data-section=depots]"), |dom| {
            dom.el(el("h4.settings__panel-title"), |dom| {
                dom.text(self.t("settings.sectionDepots"));
            });
            dom.el(self.ui.el_as("settings-group-5", "div.settings-group"), |dom| {
                if dom.button(self.ui.el_as("settings-row-12", "label.settings-row[for=auto-select-depots-toggle]").state(HAS_CHECKED, self.ui.checked("auto-select-depots-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.autoSelectDepots"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.autoSelectDepotsHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#auto-select-depots-toggle[type=checkbox]").state(CHECKED, self.ui.checked("auto-select-depots-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("auto-select-depots-toggle");
                }
                if dom.button(self.ui.el_as("settings-row-13", "label.settings-row[for=latest-manifests-toggle]").state(HAS_CHECKED, self.ui.checked("latest-manifests-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.latestManifests"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.latestManifestsHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#latest-manifests-toggle[type=checkbox]").state(CHECKED, self.ui.checked("latest-manifests-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("latest-manifests-toggle");
                }
            });
            dom.el(self.ui.el("div.auto-select-options#auto-select-options"), |dom| {
                dom.el(self.ui.el_as("settings-title-2", "h5.settings__group-title"), |dom| {
                    dom.text(self.t("settings.groupLikeSteam"));
                });
                dom.el(self.ui.el_as("settings-group-6", "div.settings-group"), |dom| {
                    if dom.button(self.ui.el_as("settings-row-14", "label.settings-row[for=auto-start-download-toggle]").state(HAS_CHECKED, self.ui.checked("auto-start-download-toggle")), |dom| {
                        dom.el(el("span.settings-row__text"), |dom| {
                            dom.el(el("span.settings-row__label"), |dom| {
                                dom.text(self.t("settings.autoStartDownload"));
                            });
                            dom.el(el("span.settings-row__hint"), |dom| {
                                dom.text(self.t("settings.autoStartDownloadHint"));
                            });
                        });
                        dom.el(el("span.toggle-switch"), |dom| {
                            dom.empty(self.ui.el("input#auto-start-download-toggle[type=checkbox]").state(CHECKED, self.ui.checked("auto-start-download-toggle")));
                            dom.empty(el("span.toggle-switch__slider"));
                        });
                    }).clicked() {
                        self.toggle("auto-start-download-toggle");
                    }
                    if dom.button(self.ui.el_as("settings-row-15", "label.settings-row[for=include-dlc-toggle]").state(HAS_CHECKED, self.ui.checked("include-dlc-toggle")), |dom| {
                        dom.el(el("span.settings-row__text"), |dom| {
                            dom.el(el("span.settings-row__label"), |dom| {
                                dom.text(self.t("settings.includeDlc"));
                            });
                            dom.el(el("span.settings-row__hint"), |dom| {
                                dom.text(self.t("settings.includeDlcHint"));
                            });
                        });
                        dom.el(el("span.toggle-switch"), |dom| {
                            dom.empty(self.ui.el("input#include-dlc-toggle[type=checkbox]").state(CHECKED, self.ui.checked("include-dlc-toggle")));
                            dom.empty(el("span.toggle-switch__slider"));
                        });
                    }).clicked() {
                        self.toggle("include-dlc-toggle");
                    }
                    dom.el(self.ui.el_as("settings-row-16", "div.settings-row.settings-row--wrap"), |dom| {
                        dom.el(el("span.settings-row__text"), |dom| {
                            dom.el(el("span.settings-row__label"), |dom| {
                                dom.text(self.t("settings.targetPlatform"));
                            });
                            dom.el(el("span.settings-row__hint"), |dom| {
                                dom.text(self.t("settings.targetPlatformHint"));
                            });
                        });
                        dom.el(self.ui.el("div.segmented#target-platform-options"), |dom| {
                            if dom.button(self.ui.el_as("target-platform-options-", "button.segmented__option.is-active"), |dom| {
                                dom.text(self.t("settings.auto"));
                            }).clicked() {
                                self.click("target-platform-options:");
                            }
                            if dom.button(self.ui.el_as("target-platform-options-windows", "button.segmented__option"), |dom| {
                                dom.text("Windows");
                            }).clicked() {
                                self.click("target-platform-options:windows");
                            }
                            if dom.button(self.ui.el_as("target-platform-options-linux", "button.segmented__option"), |dom| {
                                dom.text("Linux");
                            }).clicked() {
                                self.click("target-platform-options:linux");
                            }
                            if dom.button(self.ui.el_as("target-platform-options-macos", "button.segmented__option"), |dom| {
                                dom.text("macOS");
                            }).clicked() {
                                self.click("target-platform-options:macos");
                            }
                        });
                    });
                    dom.el(self.ui.el_as("settings-row-17", "div.settings-row"), |dom| {
                        dom.el(el("span.settings-row__text"), |dom| {
                            dom.el(el("span.settings-row__label"), |dom| {
                                dom.text(self.t("settings.gameLanguage"));
                            });
                        });
                        dom.el(self.ui.el("div.history-sort#game-language"), |dom| {
                            if dom.button(self.ui.el("button.history-sort__button#game-language-button"), |dom| {
                                dom.el(self.ui.el("span.history-sort__label#game-language-label"), |dom| {
                                    self.fill("game-language-label", dom);
                                });
                                dom.icon_with(el("svg.history-sort__chevron").size(14.0, 14.0), icons::CHEVRON_DOWN, 2.0, true);
                            }).clicked() {
                                self.click("game-language-button");
                            }
                            dom.el(self.ui.el("ul.history-sort__menu.history-sort__menu--scroll.hidden#game-language-menu"), |dom| {
                                self.fill("game-language-menu", dom);
                            });
                        });
                    });
                });
            });
        });
    }

    pub(crate) fn view_settings_panel_sources(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el_as("settings-panel-sources", "section.settings__panel[data-section=sources]"), |dom| {
            dom.el(el("h4.settings__panel-title"), |dom| {
                dom.text(self.t("settings.sectionSources"));
            });
            dom.el(self.ui.el_as("settings-group-7", "div.settings-group"), |dom| {
                dom.el(self.ui.el_as("settings-row-18", "div.settings-row.settings-row--stack"), |dom| {
                    dom.el(el("div.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.depotSources"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            self.html_key("settings.depotSourcesHint", dom);
                        });
                    });
                    dom.el(self.ui.el("ul.sources-list#sources-list").state(crate::style::selector::EMPTY, self.is_empty("sources-list")), |dom| {
                        self.fill("sources-list", dom);
                    });
                    dom.el(el("div.sources-add-row"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#sources-add-input[type=text]");
                            let placeholder = self.ui.attr("sources-add-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.depotSourcePlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("sources-add-input"), &placeholder, false);
                            self.input_events("sources-add-input", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--outline#btn-sources-add"), |dom| {
                            self.label("btn-sources-add", "settings.add", dom);
                        }).clicked() {
                            self.click("btn-sources-add");
                        }
                    });
                    dom.el(self.ui.el("p.error-message.hidden#sources-add-error"), |dom| {
                        self.fill("sources-add-error", dom);
                    });
                });
                dom.el(self.ui.el_as("settings-row-19", "div.settings-row.settings-row--stack"), |dom| {
                    if dom.button(el("label.settings-row__text[for=hubcap-apikey-input]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.hubcapApiKey"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            self.html_key("settings.hubcapApiKeyHint", dom);
                        });
                    }).clicked() {
                        self.label_click("hubcap-apikey-input");
                    }
                    {
                        let element = self.ui.el("input.dd-path__input#hubcap-apikey-input[type=text]");
                        let placeholder = self.ui.attr("hubcap-apikey-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.hubcapApiKeyPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("hubcap-apikey-input"), &placeholder, false);
                        self.input_events("hubcap-apikey-input", &response);
                    }
                });
                dom.el(self.ui.el_as("settings-row-20", "div.settings-row.settings-row--stack"), |dom| {
                    if dom.button(el("label.settings-row__text[for=ryuu-apikey-input]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.ryuuApiKey"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            self.html_key("settings.ryuuApiKeyHint", dom);
                        });
                    }).clicked() {
                        self.label_click("ryuu-apikey-input");
                    }
                    {
                        let element = self.ui.el("input.dd-path__input#ryuu-apikey-input[type=text]");
                        let placeholder = self.ui.attr("ryuu-apikey-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.ryuuApiKeyPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("ryuu-apikey-input"), &placeholder, false);
                        self.input_events("ryuu-apikey-input", &response);
                    }
                });
            });
            dom.el(self.ui.el_as("settings-title-3", "h5.settings__group-title"), |dom| {
                dom.text(self.t("settings.groupNetwork"));
            });
            dom.el(self.ui.el_as("settings-group-8", "div.settings-group"), |dom| {
                dom.el(self.ui.el_as("settings-row-21", "div.settings-row.settings-row--stack"), |dom| {
                    if dom.button(el("label.settings-row__text[for=proxy-input]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.proxy"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.proxyHint"));
                        });
                    }).clicked() {
                        self.label_click("proxy-input");
                    }
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#proxy-input[type=text]");
                            let placeholder = self.ui.attr("proxy-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.proxyPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("proxy-input"), &placeholder, false);
                            self.input_events("proxy-input", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-proxy-test"), |dom| {
                            self.label("btn-proxy-test", "settings.proxyTest", dom);
                        }).clicked() {
                            self.click("btn-proxy-test");
                        }
                    });
                    dom.el(self.ui.el("p.proxy-status.hidden#proxy-status"), |dom| {
                        self.fill("proxy-status", dom);
                    });
                    dom.el(self.ui.el("p.error-message.hidden#proxy-error"), |dom| {
                        self.fill("proxy-error", dom);
                    });
                });
                dom.el(self.ui.el_as("settings-row-22", "div.settings-row.settings-row--stack"), |dom| {
                    if dom.button(el("label.settings-row__text#download-server-select-label[for=download-server-select]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.downloadServer"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.downloadServerHint"));
                        });
                    }).clicked() {
                        self.label_click("download-server-select");
                    }
                    self.select(dom, self.ui.el("select#download-server-select"), "download-server-select", &[("auto", "settings.downloadServerAuto", "Automatic (like Steam)"), ("custom", "settings.downloadServerCustom", "Own server…")]);
                    dom.el(self.ui.el("div.hidden#download-server-custom"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#download-server-input[type=text]");
                            let placeholder = self.ui.attr("download-server-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.downloadServerPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("download-server-input"), &placeholder, false);
                            self.input_events("download-server-input", &response);
                        }
                        dom.el(el("p.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.downloadServerCustomHint"));
                        });
                    });
                    dom.el(self.ui.el("p.error-message.hidden#download-server-error"), |dom| {
                        self.fill("download-server-error", dom);
                    });
                });
            });
        });
    }

    pub(crate) fn view_settings_panel_steam(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el_as("settings-panel-steam", "section.settings__panel[data-section=steam]"), |dom| {
            dom.el(el("h4.settings__panel-title"), |dom| {
                dom.text(self.t("settings.sectionSteam"));
            });
            dom.el(self.ui.el_as("settings-group-9", "div.settings-group"), |dom| {
                dom.el(self.ui.el_as("settings-row-23", "div.settings-row.settings-row--stack"), |dom| {
                    if dom.button(el("label.settings-row__text[for=steam-webapi-key-input]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.steamWebApiKey"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            self.html_key("settings.steamWebApiKeyHint", dom);
                        });
                    }).clicked() {
                        self.label_click("steam-webapi-key-input");
                    }
                    {
                        let element = self.ui.el("input.dd-path__input#steam-webapi-key-input[type=password]");
                        let placeholder = self.ui.attr("steam-webapi-key-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.steamWebApiKeyPlaceholder").to_string());
                        let response = dom.input_with(element, self.ui.value_mut("steam-webapi-key-input"), &placeholder, true);
                        self.input_events("steam-webapi-key-input", &response);
                    }
                });
                dom.el(self.ui.el_as("settings-row-24", "div.settings-row.settings-row--stack"), |dom| {
                    if dom.button(el("label.settings-row__text[for=steam-path-input]"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.steamPath"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.steamPathHint"));
                        });
                    }).clicked() {
                        self.label_click("steam-path-input");
                    }
                    dom.el(el("div.dd-path__input-wrap.dd-path__input-wrap--with-btn"), |dom| {
                        {
                            let element = self.ui.el("input.dd-path__input#steam-path-input[type=text]");
                            let placeholder = self.ui.attr("steam-path-input", "placeholder").map(str::to_string).unwrap_or_else(|| self.t("settings.steamPathPlaceholder").to_string());
                            let response = dom.input_with(element, self.ui.value_mut("steam-path-input"), &placeholder, false);
                            self.input_events("steam-path-input", &response);
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline#btn-steam-path-browse"), |dom| {
                            self.label("btn-steam-path-browse", "shortcut.browse", dom);
                        }).clicked() {
                            self.click("btn-steam-path-browse");
                        }
                    });
                    dom.el(self.ui.el("p.proxy-status.hidden#steam-path-status"), |dom| {
                        self.fill("steam-path-status", dom);
                    });
                });
            });
            dom.el(self.ui.el_as("settings-title-4", "h5.settings__group-title"), |dom| {
                dom.text(self.t("account.title"));
            });
            dom.el(self.ui.el("div.settings-group#account-group"), |dom| {
                dom.el(self.ui.el_as("settings-row-25", "div.settings-row"), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(self.ui.el("span.settings-row__label#account-status"), |dom| {
                            self.label("account-status", "account.signedOut", dom);
                        });
                        dom.el(self.ui.el("span.settings-row__hint#account-detail"), |dom| {
                            self.label("account-detail", "account.signedOutHint", dom);
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("account.note"));
                        });
                    });
                    dom.el(el("span.account-actions"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--small.btn--primary#btn-account-login"), |dom| {
                            self.label("btn-account-login", "account.signIn", dom);
                        }).clicked() {
                            self.click("btn-account-login");
                        }
                        if dom.button(self.ui.el("button.btn.btn--small.btn--outline.hidden#btn-account-logout"), |dom| {
                            self.label("btn-account-logout", "account.signOut", dom);
                        }).clicked() {
                            self.click("btn-account-logout");
                        }
                    });
                });
            });
        });
    }

    pub(crate) fn view_settings_panel_privacy(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el_as("settings-panel-privacy", "section.settings__panel[data-section=privacy]"), |dom| {
            dom.el(el("h4.settings__panel-title"), |dom| {
                dom.text(self.t("settings.sectionPrivacy"));
            });
            dom.el(self.ui.el_as("settings-group-10", "div.settings-group"), |dom| {
                if dom.button(self.ui.el_as("settings-row-26", "label.settings-row[for=telemetry-toggle]").state(HAS_CHECKED, self.ui.checked("telemetry-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.telemetry"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            self.html_key("settings.telemetryHint", dom);
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#telemetry-toggle[type=checkbox]").state(CHECKED, self.ui.checked("telemetry-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("telemetry-toggle");
                }
                if dom.button(self.ui.el_as("settings-row-27", "label.settings-row[for=remote-updates-toggle]").state(HAS_CHECKED, self.ui.checked("remote-updates-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.remoteUpdates"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.remoteUpdatesHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#remote-updates-toggle[type=checkbox]").state(CHECKED, self.ui.checked("remote-updates-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("remote-updates-toggle");
                }
                if dom.button(self.ui.el("label.settings-row#behavior-row[for=behavior-toggle]").state(HAS_CHECKED, self.ui.checked("behavior-toggle")), |dom| {
                    dom.el(el("span.settings-row__text"), |dom| {
                        dom.el(el("span.settings-row__label"), |dom| {
                            dom.text(self.t("settings.telemetryBehavior"));
                        });
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.telemetryBehaviorHint"));
                        });
                    });
                    dom.el(el("span.toggle-switch"), |dom| {
                        dom.empty(self.ui.el("input#behavior-toggle[type=checkbox]").state(CHECKED, self.ui.checked("behavior-toggle")));
                        dom.empty(el("span.toggle-switch__slider"));
                    });
                }).clicked() {
                    self.toggle("behavior-toggle");
                }
            });
            dom.el(self.ui.el_as("settings-title-5", "h5.settings__group-title"), |dom| {
                dom.text(self.t("settings.debugInfo"));
            });
            dom.el(self.ui.el_as("settings-group-11", "div.settings-group"), |dom| {
                dom.el(self.ui.el_as("settings-row-28", "div.settings-row.settings-row--stack"), |dom| {
                    dom.el(self.ui.el("div.build-info#build-info"), |dom| {
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoChannel"));
                            });
                            dom.el(el("span.build-info__val"), |dom| {
                                dom.el(self.ui.el("span.build-info__badge#build-info-channel"), |dom| {
                                    self.fill("build-info-channel", dom);
                                });
                            });
                        });
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoVersion"));
                            });
                            dom.el(self.ui.el("span.build-info__val#build-info-version"), |dom| {
                                self.fill("build-info-version", dom);
                            });
                        });
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoCommit"));
                            });
                            dom.el(self.ui.el("span.build-info__val#build-info-sha"), |dom| {
                                self.fill("build-info-sha", dom);
                            });
                        });
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoBuildDate"));
                            });
                            dom.el(self.ui.el("span.build-info__val#build-info-date"), |dom| {
                                self.fill("build-info-date", dom);
                            });
                        });
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoProfile"));
                            });
                            dom.el(self.ui.el("span.build-info__val#build-info-profile"), |dom| {
                                self.fill("build-info-profile", dom);
                            });
                        });
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoPlatform"));
                            });
                            dom.el(self.ui.el("span.build-info__val#build-info-platform"), |dom| {
                                self.fill("build-info-platform", dom);
                            });
                        });
                        dom.el(el("div.build-info__row"), |dom| {
                            dom.el(el("span.build-info__key"), |dom| {
                                dom.text(self.t("settings.debugInfoDiagnosticId"));
                            });
                            dom.el(self.ui.el("span.build-info__val#build-info-diagnostic"), |dom| {
                                self.fill("build-info-diagnostic", dom);
                            });
                        });
                    });
                    dom.el(el("div.settings-row__actions"), |dom| {
                        if dom.button(self.ui.el("button.btn.btn--outline.btn--small#btn-copy-build-info"), |dom| {
                            self.label("btn-copy-build-info", "settings.copyDebugInfo", dom);
                        }).clicked() {
                            self.click("btn-copy-build-info");
                        }
                        if dom.button(self.ui.el("button.btn.btn--outline.btn--small#btn-settings-report-bug"), |dom| {
                            self.label("btn-settings-report-bug", "bugReport.title", dom);
                        }).clicked() {
                            self.click("btn-settings-report-bug");
                        }
                        dom.el(el("span.settings-row__hint"), |dom| {
                            dom.text(self.t("settings.debugInfoHint"));
                        });
                    });
                });
            });
        });
    }
}
