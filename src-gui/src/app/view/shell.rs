#![allow(unused_imports, clippy::all)]

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;
use crate::style::selector::{CHECKED, HAS_CHECKED};

impl App {
    pub(crate) fn view_body(&mut self, dom: &mut Dom<'_>) {
        dom.el(el("div.app-shell"), |dom| {
            self.view_title_bar(dom);
            dom.el(el("div.scroll-wrapper"), |dom| {
                dom.el(self.ui.el_as("app", "div.app"), |dom| {
                    self.view_header(dom);
                    dom.el(self.ui.el("div.app-banner.app-banner--warning.hidden#offline-banner"), |dom| {
                        dom.el(el("span"), |dom| {
                            dom.text(self.t("remote.offline"));
                        });
                    });
                    dom.el(self.ui.el("div.app-notices#remote-notices").state(crate::style::selector::EMPTY, self.is_empty("remote-notices")), |dom| {
                        self.fill("remote-notices", dom);
                    });
                    self.view_dotnet_warning(dom);
                    self.view_steps_indicator(dom);
                    self.view_step_upload(dom);
                    self.view_step_select(dom);
                    self.view_step_progress(dom);
                    self.view_step_shortcut(dom);
                    self.view_step_steam_library(dom);
                    self.view_step_emulator(dom);
                });
            });
            dom.el(self.ui.el("div.app-toast.hidden#app-toast"), |dom| {
                self.fill("app-toast", dom);
            });
        });
    }

    pub(crate) fn view_title_bar(&mut self, dom: &mut Dom<'_>) {
        let dragged = dom.hit(self.ui.el("div.title-bar#title-bar"), egui::Sense::click_and_drag(), |dom| {
            dom.el(el("div.title-bar__drag-region"), |dom| {
                dom.el(el("div.title-bar__icon"), |dom| {
                    self.image(dom, el("img").size(16.0, 16.0), "/assets/icon-small.svg");
                });
                dom.el(el("span.title-bar__text"), |dom| {
                    dom.text(self.t("appName"));
                });
            });
            dom.el(el("div.title-bar__controls"), |dom| {
                if dom.button(self.ui.el("button.title-bar__btn.title-bar__btn--minimize#btn-minimize").title_or(self.t("titleBar.minimize")), |dom| {
                    dom.icon(el("svg").size(12.0, 12.0), icons::WIN_MINIMIZE);
                }).clicked() {
                    self.click("btn-minimize");
                }
                if dom.button(self.ui.el("button.title-bar__btn.title-bar__btn--maximize#btn-maximize").title_or(self.t("titleBar.maximize")), |dom| {
                    dom.icon(el("svg").size(12.0, 12.0), icons::WIN_MAXIMIZE);
                }).clicked() {
                    self.click("btn-maximize");
                }
                if dom.button(self.ui.el("button.title-bar__btn.title-bar__btn--close#btn-close").title_or(self.t("titleBar.close")), |dom| {
                    dom.icon(el("svg").size(12.0, 12.0), icons::WIN_CLOSE);
                }).clicked() {
                    self.click("btn-close");
                }
            });
        });
        self.dragged("title-bar", dragged.and_then(|out| out.response));
    }

    pub(crate) fn view_header(&mut self, dom: &mut Dom<'_>) {
        dom.el(el("header.header"), |dom| {
            dom.el(el("h1.header__title"), |dom| {
                dom.icon_with(el("svg.header__icon"), icons::DOWNLOAD, 2.0, false);
                dom.el(el("span.gradient-text"), |dom| {
                    dom.text(self.t("appName"));
                });
            });
            dom.el(el("p.header__subtitle"), |dom| {
                dom.text(self.t("header.subtitle"));
            });
            dom.el(el("div.header__actions"), |dom| {
                if dom.button(self.ui.el("button.btn.btn--outline.header__settings-btn#btn-settings").title_or(self.t("header.settings")), |dom| {
                    dom.icon_with(el("svg").size(16.0, 16.0), icons::SETTINGS, 2.0, true);
                }).clicked() {
                    self.click("btn-settings");
                }
                if dom.button(self.ui.el("button.btn.btn--outline.header__history-btn#btn-history").title_or(self.t("header.history")), |dom| {
                    dom.icon_with(el("svg").size(16.0, 16.0), icons::CLOCK, 2.0, false);
                }).clicked() {
                    self.click("btn-history");
                }
                if dom.button(self.ui.el("button.btn.btn--outline.header__queue-btn#btn-queue").title_or(self.t("queue.title")), |dom| {
                    dom.icon_with(el("svg").size(16.0, 16.0), icons::LIST, 2.0, true);
                    dom.el(self.ui.el("span.header__badge.hidden#queue-badge"), |dom| {
                        self.fill("queue-badge", dom);
                    });
                }).clicked() {
                    self.click("btn-queue");
                }
                if dom.button(self.ui.el("button.btn.btn--outline.header__bug-btn#btn-report-bug").title_or(self.t("bugReport.title")), |dom| {
                    dom.icon_with(el("svg").size(16.0, 16.0), icons::BUG, 2.0, true);
                }).clicked() {
                    self.click("btn-report-bug");
                }
                if dom.button(self.ui.el("button.btn.btn--outline.header__theme-btn#btn-theme-toggle").title_or(self.t("header.themeToggle")), |dom| {
                    self.fill("btn-theme-toggle", dom);
                }).clicked() {
                    self.click("btn-theme-toggle");
                }
            });
        });
    }

    pub(crate) fn view_dotnet_warning(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.dotnet-warning.hidden#dotnet-warning"), |dom| {
            dom.el(el("div.dotnet-warning__content"), |dom| {
                dom.el(el("span.dotnet-warning__icon"), |dom| {
                    dom.icon_with(el("svg"), icons::ALERT_TRIANGLE, 2.0, true);
                });
                dom.el(el("span.dotnet-warning__text"), |dom| {
                    self.html_key("dotnetWarning.text", dom);
                });
                if dom.button(self.ui.el("a.btn.btn--small.dotnet-warning__btn#dotnet-install-link"), |dom| {
                    self.label("dotnet-install-link", "dotnetWarning.download", dom);
                }).clicked() {
                    self.click("dotnet-install-link");
                }
                if dom.button(self.ui.el("button.dotnet-warning__dismiss#dotnet-warning-dismiss").title_or(self.t("dotnetWarning.dismiss")), |dom| {
                    self.fill("dotnet-warning-dismiss", dom);
                }).clicked() {
                    self.click("dotnet-warning-dismiss");
                }
            });
        });
    }

    pub(crate) fn view_steps_indicator(&mut self, dom: &mut Dom<'_>) {
        dom.el(self.ui.el("div.steps#steps-indicator"), |dom| {
            dom.el(self.ui.el_as("steps-item-1", "div.steps__item.active"), |dom| {
                dom.el(el("div.steps__number"), |dom| {
                    self.fill("steps-number-1", dom);
                });
                dom.el(el("div.steps__label"), |dom| {
                    dom.text(self.t("steps.upload"));
                });
            });
            dom.empty(el("div.steps__connector"));
            dom.el(self.ui.el_as("steps-item-2", "div.steps__item"), |dom| {
                dom.el(el("div.steps__number"), |dom| {
                    self.fill("steps-number-2", dom);
                });
                dom.el(el("div.steps__label"), |dom| {
                    dom.text(self.t("steps.select"));
                });
            });
            dom.empty(el("div.steps__connector"));
            dom.el(self.ui.el_as("steps-item-3", "div.steps__item"), |dom| {
                dom.el(el("div.steps__number"), |dom| {
                    self.fill("steps-number-3", dom);
                });
                dom.el(el("div.steps__label"), |dom| {
                    dom.text(self.t("steps.download"));
                });
            });
            dom.el(self.ui.el("div.steps__connector.steps__connector--shortcut.hidden#step4-connector"), |dom| {
                self.fill("step4-connector", dom);
            });
            dom.el(self.ui.el("div.steps__item.steps__item--shortcut.hidden#step4-indicator"), |dom| {
                dom.el(el("div.steps__number"), |dom| {
                    self.fill("steps-number-4", dom);
                });
                dom.el(el("div.steps__label"), |dom| {
                    dom.text(self.t("steps.shortcut"));
                });
            });
            dom.el(self.ui.el("div.steps__connector.steps__connector--steam.hidden#step6-connector"), |dom| {
                self.fill("step6-connector", dom);
            });
            dom.el(self.ui.el("div.steps__item.steps__item--steam.hidden#step6-indicator"), |dom| {
                dom.el(el("div.steps__number"), |dom| {
                    self.fill("steps-number-6", dom);
                });
                dom.el(el("div.steps__label"), |dom| {
                    dom.text(self.t("steps.steamLibrary"));
                });
            });
            dom.el(self.ui.el("div.steps__connector.steps__connector--emulator#step5-connector"), |dom| {
                self.fill("step5-connector", dom);
            });
            dom.el(self.ui.el("div.steps__item.steps__item--emulator#step5-indicator"), |dom| {
                dom.el(el("div.steps__number"), |dom| {
                    self.fill("steps-number-5", dom);
                });
                dom.el(el("div.steps__label"), |dom| {
                    dom.text(self.t("steps.emulator"));
                });
            });
        });
    }
}
