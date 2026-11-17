//! WeCom-style RSS reader built with Native Shell in pure Rust.
//! One component tree is shared by macOS, Windows, Linux/GPUI, iOS,
//! Android and the WebAssembly web build.

mod rss;

use std::time::Duration;

use nsh_core::prelude::*;
use nsh_core::time::unix_ms;

use rss::{FeedItem, Source};

const REFRESH_SECONDS: u64 = 5 * 60;

// Touch-target sizes. Apple HIG / Material both ask for >=44pt / >=48dp;
// 48 is a safe common floor for tappable controls on phones and tablets.
const TOUCH: f32 = 48.0;
const TAB_H: f32 = 56.0;

const PAGE: u32 = 0xF5F6F7;
const PANEL: u32 = 0xFFFFFF;
const RAIL: u32 = 0x2E3238;
const RAIL_ACTIVE: u32 = 0x3D434B;
const TEXT: u32 = 0x1F2329;
const MUTED: u32 = 0x8A9099;
const BORDER: u32 = 0xE5E6EB;
const HOVER: u32 = 0xF2F3F5;
const SELECTED: u32 = 0xE8F3FF;
const ACCENT: u32 = 0x07C160;
const BLUE: u32 = 0x2E7BE6;

/// The UI set exposed to the host (`wecom_reader/native`).
#[native_module("wecom_reader/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

type Items = Signal<Vec<FeedItem>>;

#[component]
pub fn App() -> impl IntoElement {
    let v2ex_items = signal(Vec::<FeedItem>::new());
    let linux_items = signal(Vec::<FeedItem>::new());
    let active_source = signal(Source::V2ex);
    let selected = signal(None::<FeedItem>);
    let status = signal("正在连接数据源…".to_owned());
    let updated_at = signal("尚未刷新".to_owned());
    let busy = signal(false);

    refresh_all(v2ex_items, linux_items, status, updated_at, busy);
    set_interval(Duration::from_secs(REFRESH_SECONDS), move || {
        refresh_all(v2ex_items, linux_items, status, updated_at, busy)
    });

    let wide = move || window().width.0 >= 860.0;
    // Touch devices are always mobile; a narrow desktop window (or a phone
    // browser whose pointer makes `idiom()` report Desktop) benefits from the
    // single-column layout too.
    let mobile = move || !is_desktop() || window().width.0 < 700.0;
    let show_detail = move || wide() || selected.get().is_some();
    let show_list = move || wide() || selected.get().is_none();

    // Android's hardware/system Back key: on a phone the detail view is a
    // pushed screen, so Back returns to the list instead of quitting the app.
    let back_selected = selected;
    on_back_press(move || {
        if back_selected.get().is_some() {
            back_selected.set(None);
            true // handled: stay in the app, go back to the list
        } else {
            false // nothing to pop; let the platform exit
        }
    });

    view()
        .flex_1()
        .flex_col()
        .bg(rgb(PAGE))
        .text_color(rgb(TEXT))
        // Phones/tablets: stacked single-pane with a thumb-reachable tab bar.
        // Desktop: keep the familiar three-column layout.
        .child(when(
            move || mobile(),
            move || {
                mobile_shell(
                    active_source,
                    selected,
                    v2ex_items,
                    linux_items,
                    status,
                    updated_at,
                    busy,
                )
            },
        ))
        .child(when(
            move || !mobile(),
            move || {
                desktop_shell(
                    active_source,
                    selected,
                    v2ex_items,
                    linux_items,
                    status,
                    updated_at,
                    busy,
                    wide,
                    show_list,
                    show_detail,
                )
            },
        ))
}

/// Desktop: narrow icon rail + list + detail, side by side.
fn desktop_shell(
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
    v2ex_items: Items,
    linux_items: Items,
    status: Signal<String>,
    updated_at: Signal<String>,
    busy: Signal<bool>,
    wide: impl Fn() -> bool + Copy + 'static,
    show_list: impl Fn() -> bool + Copy + 'static,
    show_detail: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .flex_row()
        .p_safe_area()
        .child(rail(
            source,
            selected,
            v2ex_items,
            linux_items,
            status,
            updated_at,
            busy,
            wide,
        ))
        .child(when(show_list, move || {
            list_panel(
                v2ex_items,
                linux_items,
                source,
                selected,
                status,
                updated_at,
                busy,
            )
        }))
        .child(when(show_detail, move || {
            detail_panel(selected, busy, wide)
        }))
}

/// Mobile: content fills the screen; navigation lives in a bottom tab bar
/// within easy thumb reach. The detail view replaces the list (a pushed
/// screen) and offers a full-width Back affordance in its top bar.
fn mobile_shell(
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
    v2ex_items: Items,
    linux_items: Items,
    status: Signal<String>,
    updated_at: Signal<String>,
    busy: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .pt_safe_area() // keep clear of the status bar / notch
        .child(
            view()
                .flex_1()
                .flex_col()
                .child(when(
                    move || selected.get().is_none(),
                    move || {
                        mobile_list(
                            v2ex_items,
                            linux_items,
                            source,
                            selected,
                            status,
                            updated_at,
                            busy,
                        )
                    },
                ))
                .child(when(
                    move || selected.get().is_some(),
                    move || mobile_detail(selected),
                )),
        )
        .child(mobile_tab_bar(source, selected))
}

/// The two sources as a bottom tab bar — the standard mobile navigation
/// pattern, reachable with a thumb. Each target is at least 48pt tall and the
/// bar pads for the home indicator.
fn mobile_tab_bar(source: Signal<Source>, selected: Signal<Option<FeedItem>>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .bg(rgb(PANEL))
        .border_t(px(1.0))
        .border_color(rgb(BORDER))
        .px_safe_area()
        .pb_safe_area_plus(px(6.0))
        .child(mobile_tab(Source::V2ex, source, selected))
        .child(mobile_tab(Source::LinuxDo, source, selected))
}

fn mobile_tab(
    target: Source,
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
) -> impl IntoElement {
    let active = move || source.get() == target;
    let color = if target == Source::V2ex {
        rgb(BLUE)
    } else {
        rgb(ACCENT)
    };
    let glyph = if target == Source::V2ex { "V" } else { "L" };
    let label = target.label();
    view()
        .flex_1()
        .h(px(TAB_H))
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .active(|s| s.opacity(0.6))
        .on_press(move || {
            source.set(target);
            selected.set(None); // leaving a source returns to its list
        })
        .child(
            view()
                .w(px(26.0))
                .h(px(26.0))
                .rounded(px(8.0))
                .bg(move || if active() { color } else { rgb(0xC8CDD3) })
                .items_center()
                .justify_center()
                .child(
                    text(glyph)
                        .text_size(px(14.0))
                        .text_color(rgb(PANEL))
                        .font_weight(FontWeight::BOLD),
                ),
        )
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || if active() { color } else { rgb(MUTED) }),
        )
}

/// Mobile list: one full-width column. Header carries the title and a
/// comfortably-sized refresh button; rows are tall enough to tap reliably.
fn mobile_list(
    v2ex_items: Items,
    linux_items: Items,
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
    status: Signal<String>,
    updated_at: Signal<String>,
    busy: Signal<bool>,
) -> impl IntoElement {
    let items = move || {
        if source.get() == Source::V2ex {
            v2ex_items.get()
        } else {
            linux_items.get()
        }
    };
    view()
        .flex_1()
        .flex_col()
        .bg(rgb(PANEL))
        // Top navigation bar, padded from the notch by the shell above.
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px_4()
                .py_3()
                .border_b(px(1.0))
                .border_color(rgb(BORDER))
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex_col()
                        .gap_1()
                        .child(
                            text(move || source.get().label().to_owned())
                                .text_size(px(20.0))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            text(move || {
                                if busy.get() {
                                    "同步中…".to_owned()
                                } else {
                                    format!("最近同步 · {}", updated_at.get())
                                }
                            })
                            .text_size(px(11.0))
                            .text_color(rgb(MUTED)),
                        ),
                )
                .child(
                    view()
                        .flex_shrink(0.0)
                        .w(px(TOUCH))
                        .h(px(TOUCH))
                        .rounded_lg()
                        .bg(rgb(HOVER))
                        .items_center()
                        .justify_center()
                        .active(|s| s.opacity(0.6))
                        .on_press(move || {
                            refresh_all(v2ex_items, linux_items, status, updated_at, busy)
                        })
                        .child(text("↻").text_size(px(24.0)).text_color(rgb(BLUE))),
                ),
        )
        .child(scroll().flex_1().child(each(
            items,
            |item| item.id.clone(),
            move |item, _| feed_row(item, selected),
        )))
}

/// Mobile detail: a pushed screen. A full-width Back row at the top and a
/// large "Open original" button pinned to the bottom, both thumb-friendly.
fn mobile_detail(selected: Signal<Option<FeedItem>>) -> impl IntoElement {
    let item = selected.get().unwrap_or_else(|| FeedItem {
        id: String::new(),
        source: Source::V2ex,
        title: String::new(),
        link: String::new(),
        date: String::new(),
        author: String::new(),
        summary: String::new(),
        content: String::new(),
    });
    let title = item.title.clone();
    let date = item.date.clone();
    let source_label = item.source.label().to_owned();
    let author = item.author.clone();
    let content = if item.content.is_empty() {
        item.summary.clone()
    } else {
        item.content.clone()
    };
    let url = item.link.clone();
    let has_link = !url.is_empty();

    view()
        .flex_1()
        .flex_col()
        .bg(rgb(PANEL))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py_2()
                .min_h(px(TOUCH))
                .border_b(px(1.0))
                .border_color(rgb(BORDER))
                .child(
                    view()
                        .w(px(TOUCH))
                        .h(px(TOUCH))
                        .rounded_lg()
                        .items_center()
                        .justify_center()
                        .active(|s| s.opacity(0.6))
                        .on_press(move || selected.set(None))
                        .child(text("‹").text_size(px(30.0)).text_color(rgb(BLUE))),
                )
                .child(
                    view()
                        .flex_1()
                        .flex_col()
                        .gap_1()
                        .child(
                            text(title)
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            text({
                                if author.is_empty() {
                                    format!("{source_label} · {date}")
                                } else {
                                    format!("{source_label} · {author} · {date}")
                                }
                            })
                            .text_size(px(12.0))
                            .text_color(rgb(MUTED)),
                        ),
                ),
        )
        .child(
            scroll().flex_1().child(
                view().px_4().py_4().child(
                    text(if content.is_empty() {
                        "这条动态暂无正文，点击“打开原文”查看详情。".to_owned()
                    } else {
                        content
                    })
                    .text_size(px(16.0))
                    .text_color(rgb(TEXT)),
                ),
            ),
        )
        .child(when(
            move || has_link,
            move || {
                let button_url = url.clone();
                view()
                    .mx_4()
                    .mb_4()
                    .h(px(TOUCH))
                    .rounded_lg()
                    .bg(rgb(BLUE))
                    .items_center()
                    .justify_center()
                    .active(|s| s.opacity(0.8))
                    .on_press(move || {
                        let _ = open_url(button_url.clone());
                    })
                    .child(
                        text("打开原文")
                            .text_size(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(PANEL)),
                    )
            },
        ))
}

fn rail(
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
    v2ex_items: Items,
    linux_items: Items,
    status: Signal<String>,
    updated_at: Signal<String>,
    busy: Signal<bool>,
    wide: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    view()
        .w(move || if wide() { px(56.0) } else { px(48.0) })
        .flex_col()
        .items_center()
        .gap_3()
        .py_5()
        .bg(rgb(RAIL))
        .child(
            view()
                .w(px(34.0))
                .h(px(34.0))
                .rounded_lg()
                .bg(rgb(ACCENT))
                .items_center()
                .justify_center()
                .child(text("W").text_size(px(17.0)).text_color(rgb(PANEL))),
        )
        .child(rail_button("V", Source::V2ex, source, selected, || {
            rgb(BLUE)
        }))
        .child(rail_button("L", Source::LinuxDo, source, selected, || {
            rgb(ACCENT)
        }))
        .child(view().flex_1())
        .child(
            view()
                .w(px(38.0))
                .h(px(38.0))
                .rounded_lg()
                .items_center()
                .justify_center()
                .hover(|s| s.bg(rgb(RAIL_ACTIVE)))
                .active(|s| s.opacity(0.7))
                .on_press(move || refresh_all(v2ex_items, linux_items, status, updated_at, busy))
                .child(text("↻").text_size(px(20.0)).text_color(rgb(0xB7BDC6))),
        )
}

fn rail_button(
    glyph: &'static str,
    target: Source,
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
    color: impl Fn() -> nsh_core::Rgba + Copy + 'static,
) -> impl IntoElement {
    let is_active = move || source.get() == target;
    let on = selected;
    view()
        .w(px(38.0))
        .h(px(38.0))
        .rounded_lg()
        .items_center()
        .justify_center()
        .bg(move || {
            if is_active() {
                color()
            } else {
                rgb(RAIL_ACTIVE)
            }
        })
        .hover(|s| s.opacity(0.85))
        .active(|s| s.opacity(0.7))
        .on_press(move || {
            source.set(target);
            on.set(None);
        })
        .child(
            text(glyph)
                .text_size(px(17.0))
                .text_color(rgb(PANEL))
                .font_weight(FontWeight::BOLD),
        )
}

fn list_panel(
    v2ex_items: Items,
    linux_items: Items,
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
    status: Signal<String>,
    updated_at: Signal<String>,
    busy: Signal<bool>,
) -> impl IntoElement {
    let items = move || {
        if source.get() == Source::V2ex {
            v2ex_items.get()
        } else {
            linux_items.get()
        }
    };

    view()
        .w(px(300.0))
        .flex_col()
        .bg(rgb(PANEL))
        .border_r(px(1.0))
        .border_color(rgb(BORDER))
        .child(
            view()
                .px_4()
                .pt_4()
                .pb_3()
                .gap_2()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            text(move || source.get().label().to_owned())
                                .text_size(px(18.0))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            text(move || {
                                if busy.get() {
                                    "同步中…".to_owned()
                                } else {
                                    status.get()
                                }
                            })
                            .text_size(px(11.0))
                            .text_color(rgb(MUTED)),
                        ),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .child(source_tab("V2EX", Source::V2ex, source, selected))
                        .child(source_tab("LINUX DO", Source::LinuxDo, source, selected))
                        .child(view().flex_1())
                        .child(
                            view()
                                .px_2()
                                .py_1()
                                .rounded(px(6.0))
                                .hover(|s| s.bg(rgb(HOVER)))
                                .active(|s| s.opacity(0.65))
                                .on_press(move || {
                                    refresh_all(v2ex_items, linux_items, status, updated_at, busy)
                                })
                                .child(text("刷新").text_size(px(12.0)).text_color(rgb(BLUE))),
                        ),
                )
                .child(
                    text(move || format!("最近同步 · {}", updated_at.get()))
                        .text_size(px(11.0))
                        .text_color(rgb(MUTED)),
                ),
        )
        .child(scroll().flex_1().child(each(
            items,
            |item| item.id.clone(),
            move |item, _| feed_row(item, selected),
        )))
}

fn source_tab(
    label: &'static str,
    target: Source,
    source: Signal<Source>,
    selected: Signal<Option<FeedItem>>,
) -> impl IntoElement {
    let active = move || source.get() == target;
    view()
        .px_2()
        .py_1()
        .rounded(px(6.0))
        .bg(move || if active() { rgb(SELECTED) } else { rgb(PANEL) })
        .hover(|s| s.bg(rgb(HOVER)))
        .active(|s| s.opacity(0.7))
        .on_press(move || {
            source.set(target);
            selected.set(None);
        })
        .child(
            text(label)
                .text_size(px(12.0))
                .text_color(move || if active() { rgb(BLUE) } else { rgb(MUTED) }),
        )
}

fn feed_row(item: ReadSignal<FeedItem>, selected: Signal<Option<FeedItem>>) -> impl IntoElement {
    let current = item.get();
    let id = current.id.clone();
    let is_selected = move || {
        selected
            .get()
            .is_some_and(|value| value.id == id && value.source == current.source)
    };
    let color = if current.source == Source::V2ex {
        rgb(BLUE)
    } else {
        rgb(ACCENT)
    };
    let initial = current.title.chars().next().unwrap_or('?').to_string();
    let title = current.title;
    let summary = current.summary;
    let date = current.date;

    // Rows are the primary tap target: >=64px on touch devices and narrow
    // windows, tighter on a wide desktop where a mouse allows denser rows.
    let touch = move || !is_desktop() || window().width.0 < 700.0;

    view()
        .flex_row()
        .gap_3()
        .px_4()
        .py(move || if touch() { px(14.0) } else { px(12.0) })
        .min_h(move || if touch() { px(64.0) } else { px(0.0) })
        .bg(move || {
            if is_selected() {
                rgb(SELECTED)
            } else {
                rgb(PANEL)
            }
        })
        .hover(|s| s.bg(rgb(HOVER)))
        .active(|s| s.opacity(0.8))
        .on_press(move || selected.set(Some(item.get())))
        .child(
            view()
                .w(move || if touch() { px(40.0) } else { px(36.0) })
                .h(move || if touch() { px(40.0) } else { px(36.0) })
                .rounded(px(8.0))
                .bg(color)
                .items_center()
                .justify_center()
                .child(
                    text(initial)
                        .text_size(move || if touch() { px(17.0) } else { px(15.0) })
                        .text_color(rgb(PANEL)),
                ),
        )
        .child(
            view()
                .flex_1()
                .gap_1()
                .child(
                    text(title)
                        .text_size(move || if touch() { px(16.0) } else { px(14.0) })
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_overflow(nsh_core::EllipsizeMode::Tail)
                        .text_color(rgb(TEXT)),
                )
                .child(
                    text(summary)
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED))
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(text(date).text_size(px(11.0)).text_color(rgb(MUTED))),
        )
}

fn detail_panel(
    selected: Signal<Option<FeedItem>>,
    _busy: Signal<bool>,
    wide: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    view().flex_1().flex_col().bg(rgb(PAGE)).child(
        when(
            move || selected.get().is_some(),
            move || {
                let item = selected.get().unwrap_or_else(|| FeedItem {
                    id: String::new(),
                    source: Source::V2ex,
                    title: String::new(),
                    link: String::new(),
                    date: String::new(),
                    author: String::new(),
                    summary: String::new(),
                    content: String::new(),
                });
                detail_content(item, selected, wide)
            },
        )
        .otherwise(|| {
            view().flex_1().items_center().justify_center().child(
                text("选择左侧的一条消息，查看正文详情")
                    .text_size(px(15.0))
                    .text_color(rgb(MUTED)),
            )
        }),
    )
}

fn detail_content(
    item: FeedItem,
    selected: Signal<Option<FeedItem>>,
    wide: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    let title = item.title.clone();
    let date = item.date.clone();
    let source_label = item.source.label().to_owned();
    let author = item.author.clone();
    let content = if item.content.is_empty() {
        item.summary.clone()
    } else {
        item.content.clone()
    };
    let has_link = !item.link.is_empty();
    let url_for_button = item.link.clone();

    view()
        .flex_1()
        .flex_col()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .px_5()
                .py_4()
                .bg(rgb(PANEL))
                .border_b(px(1.0))
                .border_color(rgb(BORDER))
                .child(when(
                    move || !wide(),
                    move || {
                        view()
                            .px_2()
                            .py_1()
                            .rounded(px(6.0))
                            .hover(|s| s.bg(rgb(HOVER)))
                            .on_press(move || selected.set(None))
                            .child(text("‹ 返回").text_size(px(14.0)).text_color(rgb(BLUE)))
                    },
                ))
                .child(
                    view()
                        .flex_1()
                        .gap_1()
                        .child(
                            text(title)
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            text(move || {
                                if author.is_empty() {
                                    format!("{source_label} · {date}")
                                } else {
                                    format!("{source_label} · {author} · {date}")
                                }
                            })
                            .text_size(px(11.0))
                            .text_color(rgb(MUTED)),
                        ),
                )
                .child(when(has_link, move || {
                    let button_url = url_for_button.clone();
                    view()
                        .px_3()
                        .py_2()
                        .rounded(px(6.0))
                        .bg(rgb(BLUE))
                        .hover(|s| s.opacity(0.88))
                        .active(|s| s.opacity(0.7))
                        .on_press(move || {
                            let _ = open_url(button_url.clone());
                        })
                        .child(text("打开原文").text_size(px(12.0)).text_color(rgb(PANEL)))
                })),
        )
        .child(
            scroll().flex_1().child(
                view().p_6().max_w(px(760.0)).child(
                    text(if content.is_empty() {
                        "这条动态暂无正文，点击“打开原文”查看详情。".to_owned()
                    } else {
                        content
                    })
                    .text_size(px(15.0))
                    .text_color(rgb(TEXT)),
                ),
            ),
        )
}

fn refresh_all(
    v2ex_items: Items,
    linux_items: Items,
    status: Signal<String>,
    updated_at: Signal<String>,
    busy: Signal<bool>,
) {
    if busy.get() {
        return;
    }
    busy.set(true);
    status.set("正在同步最新内容…".to_owned());

    spawn(async move {
        let mut failed = 0usize;
        let mut count = 0usize;

        for source in Source::ALL {
            // Try the primary feed first; if it is blocked or yields no items,
            // fall back to the mirror (native builds only — the web build's
            // Pages Function performs the same primary→fallback logic).
            let mut parsed = fetch_feed(source, source.feed_url()).await;
            #[cfg(not(target_arch = "wasm32"))]
            if parsed.is_empty() {
                if let Some(fallback) = source.feed_fallback_url() {
                    parsed = fetch_feed(source, fallback).await;
                }
            }

            if parsed.is_empty() {
                failed += 1;
            } else {
                count += parsed.len();
                match source {
                    Source::V2ex => v2ex_items.set(parsed),
                    Source::LinuxDo => linux_items.set(parsed),
                }
            }
        }

        updated_at.set(now_utc());
        status.set(if failed == 0 {
            format!("已载入 {count} 条")
        } else {
            "部分数据源暂不可用".to_owned()
        });
        busy.set(false);
    });
}

/// Fetch one feed URL and parse it into items. On the web build `url` is
/// ignored: every request goes through the same-origin Pages Function.
async fn fetch_feed(source: Source, url: &str) -> Vec<rss::FeedItem> {
    #[cfg(target_arch = "wasm32")]
    let target = format!("/api/rss?source={}", source.query_value());
    #[cfg(not(target_arch = "wasm32"))]
    let target = url.to_owned();

    match fetch(target).await {
        Ok(response) if response.ok() => rss::parse_feed(&response.text(), source),
        _ => Vec::new(),
    }
}

fn now_utc() -> String {
    let secs = (unix_ms() / 1000.0) as u64;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 96_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}
