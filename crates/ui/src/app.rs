//! Page Bibliothèque (maquette `Mix2`) et boucle de l'application.

use std::time::{Duration, Instant};

use iced::border::Radius;
use iced::font::Weight;
use iced::gradient::Linear;
use iced::keyboard::{self, key::Named, Key};
use iced::widget::scrollable::{AbsoluteOffset, Viewport};
use iced::widget::{
    button, canvas, column, container, image, mouse_area, row, scrollable, space, stack, text,
    text_input, Id,
};
use iced::{
    window, Background, Border, Color, ContentFit, Degrees, Element, Fill, Gradient, Length,
    Padding, Subscription, Task,
};
use launcher_core::{matches, InstalledGame, StoreId};
use launcher_stores::{ClientStatus, ScanResult};

use crate::bench::{fake_games, Report};
use crate::logo::Logo;
use crate::texts as t;
use crate::theme::{self, Accent, Mode};
use crate::transition::{Blocks, Kind};

const ROW_H: f32 = 50.0;
const TRANSITION: Duration = Duration::from_millis(300);
const LIST_ID: &str = "liste-jeux";

/// Options de lancement (ligne de commande).
#[derive(Debug, Clone)]
pub struct Options {
    /// Instant du tout début de `main`, pour mesurer le démarrage.
    pub started: Instant,
    /// Mode mesure : 2 000 jeux, défilement automatique, rapport écrit puis sortie.
    pub bench: bool,
    /// Ne pas scanner le PC : uniquement des jeux fictifs.
    pub demo: Option<usize>,
}

struct Transition {
    start: Instant,
    kind: Kind,
    target: usize,
    swapped: bool,
}

struct Bench {
    phase: BenchPhase,
    last: Option<Instant>,
    offset: f32,
    report: Report,
}

#[derive(PartialEq)]
enum BenchPhase {
    Scroll,
    Transition,
    Done,
}

struct App {
    opts: Options,
    games: Vec<InstalledGame>,
    visible: Vec<usize>,
    clients: Vec<(StoreId, ClientStatus)>,
    scanning: bool,
    query: String,
    store: Option<StoreId>,
    selected: Option<usize>,
    accent: Accent,
    mode: Mode,
    transition_kind: Kind,
    transition: Option<Transition>,
    scroll_y: f32,
    list_h: f32,
    toast: Option<(String, Instant)>,
    show_fps: bool,
    fps_last: Option<Instant>,
    fps: f32,
    first_frame_seen: bool,
    bench: Option<Bench>,
}

#[derive(Debug, Clone)]
enum Msg {
    Scanned(ScanResult, Duration),
    Search(String),
    Filter(Option<StoreId>),
    Select(usize),
    Play,
    Launched(Result<(), String>),
    Rescan,
    Scrolled(Viewport),
    Frame(Instant),
    Key(keyboard::Event),
    SetAccent(Accent),
    NextMode,
    NextTransition,
}

pub fn run(opts: Options) -> iced::Result {
    let mut app = iced::application(move || App::boot(opts.clone()), App::update, App::view)
        .title("Launcher")
        .subscription(App::subscription)
        .window_size((1440.0, 900.0))
        .default_font(Mode::Normal.font(Weight::Normal))
        .antialiasing(true);
    for f in theme::FONTS {
        app = app.font(f);
    }
    app.run()
}

impl App {
    fn boot(opts: Options) -> (Self, Task<Msg>) {
        let bench = opts.bench.then(|| Bench {
            phase: BenchPhase::Scroll,
            last: None,
            offset: 0.0,
            report: Report::default(),
        });
        let demo = opts.demo;
        let app = App {
            opts,
            games: Vec::new(),
            visible: Vec::new(),
            clients: Vec::new(),
            scanning: true,
            query: String::new(),
            store: None,
            selected: None,
            accent: Accent::Orange,
            mode: Mode::Normal,
            transition_kind: Kind::Dissolution,
            transition: None,
            scroll_y: 0.0,
            list_h: 700.0,
            toast: None,
            show_fps: false,
            fps_last: None,
            fps: 0.0,
            first_frame_seen: false,
            bench,
        };
        (app, scan_task(demo))
    }

    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.visible = (0..self.games.len())
            .filter(|&i| matches(&self.games[i], self.store, &q))
            .collect();
    }

    fn select(&mut self, i: usize) {
        if self.selected == Some(i) {
            return;
        }
        match self.transition_kind {
            Kind::Aucune => self.selected = Some(i),
            kind => {
                self.transition = Some(Transition {
                    start: Instant::now(),
                    kind,
                    target: i,
                    swapped: false,
                })
            }
        }
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Scanned(res, took) => {
                self.scanning = false;
                self.games = res.games;
                self.clients = res.clients;
                if self.bench.is_some() {
                    let missing = 2000usize.saturating_sub(self.games.len());
                    self.games.extend(fake_games(missing));
                    let t0 = Instant::now();
                    self.query = "ombre".into();
                    self.refilter();
                    let filter = t0.elapsed();
                    self.query.clear();
                    let n = self.games.len();
                    if let Some(b) = &mut self.bench {
                        b.report.scan = Some(took);
                        b.report.games = n;
                        b.report.filter = Some(filter);
                    }
                }
                if !res.errors.is_empty() {
                    self.toast = Some((res.errors.join(" · "), Instant::now()));
                }
                self.refilter();
                self.selected = self.visible.first().copied();
            }
            Msg::Search(q) => {
                self.query = q;
                self.refilter();
                self.scroll_y = 0.0;
                return iced::widget::operation::scroll_to(
                    LIST_ID,
                    AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }
            Msg::Filter(s) => {
                self.store = s;
                self.refilter();
            }
            Msg::Select(i) => self.select(i),
            Msg::Play => {
                if let Some(g) = self.selected.and_then(|i| self.games.get(i)) {
                    self.toast = Some((
                        format!(
                            "{} {} {} {}…",
                            t::LAUNCHING,
                            g.title,
                            t::VIA,
                            g.store.name()
                        ),
                        Instant::now(),
                    ));
                    let action = g.launch.clone();
                    return Task::perform(
                        async move { launcher_platform::launch(&action).map_err(|e| e.to_string()) },
                        Msg::Launched,
                    );
                }
            }
            Msg::Launched(Err(e)) => self.toast = Some((e, Instant::now())),
            Msg::Launched(Ok(())) => {}
            Msg::Rescan => {
                self.scanning = true;
                return scan_task(self.opts.demo);
            }
            Msg::Scrolled(v) => {
                self.scroll_y = v.absolute_offset().y;
                self.list_h = v.bounds().height;
            }
            Msg::Frame(now) => return self.on_frame(now),
            Msg::Key(keyboard::Event::KeyPressed {
                key: Key::Named(k), ..
            }) => match k {
                Named::F2 => self.show_fps = !self.show_fps,
                Named::F3 => self.mode = self.mode.next(),
                Named::F4 => self.transition_kind = self.transition_kind.next(),
                Named::F5 => self.accent = self.accent.next(),
                Named::Enter => return self.update(Msg::Play),
                Named::ArrowDown | Named::ArrowUp => {
                    let pos = self
                        .selected
                        .and_then(|s| self.visible.iter().position(|&v| v == s));
                    let next = match (k, pos) {
                        (Named::ArrowDown, Some(p)) => {
                            (p + 1).min(self.visible.len().saturating_sub(1))
                        }
                        (Named::ArrowUp, Some(p)) => p.saturating_sub(1),
                        _ => 0,
                    };
                    if let Some(&i) = self.visible.get(next) {
                        self.selected = Some(i);
                        // Garde la ligne choisie à l'écran.
                        let y = next as f32 * ROW_H;
                        if y < self.scroll_y || y + ROW_H > self.scroll_y + self.list_h {
                            let target = (y - self.list_h / 2.0).max(0.0);
                            return iced::widget::operation::scroll_to(
                                LIST_ID,
                                AbsoluteOffset { x: 0.0, y: target },
                            );
                        }
                    }
                }
                _ => {}
            },
            Msg::Key(_) => {}
            Msg::SetAccent(a) => self.accent = a,
            Msg::NextMode => self.mode = self.mode.next(),
            Msg::NextTransition => self.transition_kind = self.transition_kind.next(),
        }
        Task::none()
    }

    fn on_frame(&mut self, now: Instant) -> Task<Msg> {
        if !self.first_frame_seen {
            self.first_frame_seen = true;
            if let Some(b) = &mut self.bench {
                b.report.startup = Some(self.opts.started.elapsed());
            }
        }
        if let Some(last) = self.fps_last {
            let dt = now.saturating_duration_since(last).as_secs_f32();
            if dt > 0.0 {
                self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
            }
        }
        self.fps_last = Some(now);

        if let Some(tr) = &mut self.transition {
            let p =
                now.saturating_duration_since(tr.start).as_secs_f32() / TRANSITION.as_secs_f32();
            if p >= 0.5 && !tr.swapped {
                tr.swapped = true;
                self.selected = Some(tr.target);
            }
            if p >= 1.0 {
                self.transition = None;
            }
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(4))
        {
            self.toast = None;
        }
        self.bench_step(now)
    }

    fn bench_step(&mut self, now: Instant) -> Task<Msg> {
        if self.scanning {
            return Task::none();
        }
        let total = self.visible.len() as f32 * ROW_H;
        let list_h = self.list_h;
        let Some(b) = &mut self.bench else {
            return Task::none();
        };
        if let Some(last) = b.last {
            let dt = now.saturating_duration_since(last);
            match b.phase {
                BenchPhase::Scroll => b.report.frames.push(dt),
                BenchPhase::Transition => b.report.transition_frames.push(dt),
                BenchPhase::Done => {}
            }
        }
        b.last = Some(now);
        match b.phase {
            BenchPhase::Scroll => {
                b.offset += 24.0;
                if b.offset > total - list_h || b.report.frames.len() > 1200 {
                    b.phase = BenchPhase::Transition;
                } else {
                    return iced::widget::operation::scroll_to(
                        LIST_ID,
                        AbsoluteOffset {
                            x: 0.0,
                            y: b.offset,
                        },
                    );
                }
            }
            BenchPhase::Transition => {
                if self.transition.is_none() {
                    if b.report.transition_frames.len() > 90 {
                        b.phase = BenchPhase::Done;
                    } else {
                        let next = self
                            .visible
                            .get(b.report.transition_frames.len() % 7 + 1)
                            .copied();
                        if let Some(i) = next {
                            self.transition = Some(Transition {
                                start: now,
                                kind: Kind::Dissolution,
                                target: i,
                                swapped: false,
                            });
                        }
                    }
                }
            }
            BenchPhase::Done => {
                let text = b.report.render();
                println!("{text}");
                let path = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("mesures-jalon0.txt")))
                    .unwrap_or_else(|| "mesures-jalon0.txt".into());
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("Impossible d'écrire {} : {e}", path.display());
                }
                return iced::exit();
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Msg> {
        let animating = self.transition.is_some()
            || self.bench.is_some()
            || self.show_fps
            || self.toast.is_some()
            || !self.first_frame_seen;
        let keys = keyboard::listen().map(Msg::Key);
        if animating {
            Subscription::batch([keys, window::frames().map(Msg::Frame)])
        } else {
            keys
        }
    }

    // ----- Vue ---------------------------------------------------------------

    fn ui(&self) -> Ui {
        Ui {
            accent: self.accent.color(),
            mode: self.mode,
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        let ui = self.ui();
        let page = row![
            self.sidebar(ui),
            column![self.hero(ui), self.recent(ui)].width(Fill)
        ];
        let mut layers = stack![container(page)
            .width(Fill)
            .height(Fill)
            .style(|_| bg(theme::BG))];
        if let Some(tr) = &self.transition {
            let p = tr.start.elapsed().as_secs_f32() / TRANSITION.as_secs_f32();
            layers = layers.push(
                canvas(Blocks {
                    kind: tr.kind,
                    progress: p.min(1.0),
                    color: theme::BG,
                })
                .width(Fill)
                .height(Fill),
            );
        }
        if let Some((msg, _)) = &self.toast {
            let toast = container(ui.text(msg.as_str(), 14, Weight::Normal, theme::TEXT))
                .padding([12, 18])
                .style(move |_| panel(theme::CARD2, ui.mode.radius(12.0), Some(theme::LINE2)));
            layers = layers.push(
                container(toast)
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::Center)
                    .align_bottom(Fill)
                    .padding(28),
            );
        }
        if self.show_fps {
            let fps = container(ui.mono(format!("{:.0} images/s", self.fps), 13, ui.accent))
                .padding([6, 10])
                .style(move |_| panel(theme::CARD, ui.mode.radius(8.0), None));
            layers = layers.push(
                container(fps)
                    .width(Fill)
                    .align_right(Fill)
                    .padding(Padding::new(0.0).top(96.0).right(40.0)),
            );
        }
        layers.into()
    }

    fn sidebar(&self, ui: Ui) -> Element<'_, Msg> {
        let search = text_input(t::SEARCH, &self.query)
            .on_input(Msg::Search)
            .padding([12, 14])
            .size(13)
            .font(ui.mode.font(Weight::Normal))
            .style(move |_, _| text_input::Style {
                background: Background::Color(theme::FIELD),
                border: Border {
                    radius: ui.mode.radius(12.0).into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                icon: theme::MUTED,
                placeholder: theme::MUTED,
                value: theme::TEXT,
                selection: Color {
                    a: 0.4,
                    ..ui.accent
                },
            });
        let mut filters = row![].spacing(6);
        for (label, val) in [
            (t::ALL, None),
            ("Steam", Some(StoreId::Steam)),
            ("Epic", Some(StoreId::Epic)),
            ("GOG", Some(StoreId::Gog)),
        ] {
            filters = filters.push(ui.pill(label, self.store == val, Msg::Filter(val)));
        }

        // Liste virtualisée : seules les lignes visibles sont construites.
        let n = self.visible.len();
        let first = ((self.scroll_y / ROW_H).floor() as usize).min(n);
        let count = ((self.list_h / ROW_H).ceil() as usize + 2).min(n - first);
        let mut rows = column![space().height(first as f32 * ROW_H)];
        for &i in &self.visible[first..first + count] {
            rows = rows.push(self.list_row(ui, i));
        }
        rows = rows.push(space().height((n - first - count) as f32 * ROW_H));
        let list: Element<'_, Msg> = if n == 0 && !self.scanning {
            container(ui.text(
                if self.games.is_empty() {
                    t::NO_GAME
                } else {
                    t::NO_MATCH
                },
                13,
                Weight::Normal,
                theme::MUTED,
            ))
            .padding([8, 16])
            .into()
        } else {
            scrollable(container(rows).padding([0, 10]))
                .id(Id::new(LIST_ID))
                .on_scroll(Msg::Scrolled)
                .height(Fill)
                .into()
        };

        let count_label = if self.scanning {
            t::SCANNING.to_string()
        } else {
            format!("{} · {}", t::ALL_GAMES, n).to_uppercase()
        };
        let rescan = button(ui.text(t::RESCAN, 13, Weight::Normal, theme::SOFT))
            .on_press(Msg::Rescan)
            .width(Fill)
            .padding([14, 16])
            .style(|_, _| button::Style {
                background: None,
                ..Default::default()
            });

        container(
            column![
                column![search, filters]
                    .spacing(12)
                    .padding(Padding::new(16.0).top(20.0).bottom(14.0)),
                container(ui.text(count_label, 11, Weight::Semibold, theme::MUTED))
                    .padding([6, 16]),
                list,
                container(rescan).style(|_| container::Style {
                    border: Border {
                        width: 1.0,
                        color: theme::LINE,
                        radius: 0.0.into()
                    },
                    ..Default::default()
                }),
            ]
            .height(Fill),
        )
        .width(288)
        .height(Fill)
        .style(|_| bg(theme::SIDE))
        .into()
    }

    fn list_row(&self, ui: Ui, i: usize) -> Element<'_, Msg> {
        let g = &self.games[i];
        let active = self.selected == Some(i);
        let content = row![
            ui.cover(g, 34.0, 34.0, 8.0, false),
            ui.text(
                g.title.as_str(),
                13,
                if active {
                    Weight::Semibold
                } else {
                    Weight::Normal
                },
                theme::TEXT
            )
            .width(Fill),
            ui.mono(g.store.label(), 10, theme::MUTED),
        ]
        .spacing(12)
        .align_y(iced::Center);
        button(content)
            .on_press(Msg::Select(i))
            .width(Fill)
            .height(ROW_H)
            .padding([8, 10])
            .style(move |_, status| button::Style {
                background: match (active, status) {
                    (true, _) => Some(theme::CARD2.into()),
                    (false, button::Status::Hovered) => {
                        Some(Color::from_rgb8(0x16, 0x1A, 0x26).into())
                    }
                    _ => None,
                },
                text_color: theme::TEXT,
                border: Border {
                    radius: ui.mode.radius(10.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn hero(&self, ui: Ui) -> Element<'_, Msg> {
        let game = self.selected.and_then(|i| self.games.get(i));
        let header = row![
            canvas(Logo {
                accent: ui.accent,
                corner: ui.mode.radius(26.0) / 120.0
            })
            .width(42)
            .height(42),
            ui.nav(),
            space().width(Fill),
            self.spike_panel(ui),
            ui.chip(if self.scanning {
                t::SCANNING.to_string()
            } else {
                t::games_count(self.games.len())
            }),
        ]
        .spacing(8)
        .align_y(iced::Center);

        let (kicker, title) = match game {
            Some(g) => (
                g.last_played
                    .map(|_| t::RESUME.to_string())
                    .unwrap_or_else(|| t::INSTALLED.to_string()),
                g.title.clone(),
            ),
            None => (t::NO_GAME_HINT.to_uppercase(), t::NO_GAME.to_string()),
        };
        let play = button(ui.text(t::PLAY, 16, Weight::ExtraBold, theme::BG))
            .padding([16, 34])
            .on_press_maybe(game.map(|_| Msg::Play))
            .style(move |_, status| button::Style {
                background: Some(
                    if status == button::Status::Hovered {
                        lighten(ui.accent, 0.08)
                    } else {
                        ui.accent
                    }
                    .into(),
                ),
                text_color: theme::BG,
                border: Border {
                    radius: ui.mode.radius(14.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            });
        let mut actions = row![play].spacing(14).align_y(iced::Center);
        if let Some(g) = game {
            actions = actions
                .push(
                    container(ui.mono(g.store.label(), 12, theme::BG))
                        .padding([5, 9])
                        .style(move |_| panel(theme::TEXT, ui.mode.radius(6.0), None)),
                )
                .push(space().width(Fill))
                .push(ui.stat(t::PLAYTIME, playtime(g.playtime_minutes)))
                .push(ui.stat(t::SIZE, size(g.size_bytes)));
        }

        let body = column![
            header,
            space().height(Fill),
            ui.text(
                kicker,
                12,
                Weight::Semibold,
                Color::from_rgb8(0xE8, 0xEA, 0xF2)
            ),
            ui.text(title, 60, Weight::ExtraBold, theme::TEXT),
            actions,
        ]
        .spacing(14)
        .padding([32, 40]);

        let (c1, c2) = game
            .map(|g| theme::cover_colors(&g.title))
            .unwrap_or((theme::CARD2, theme::BG));
        let mut layers = stack![container(space()).width(Fill).height(Fill).style(move |_| {
            if ui.mode.is_pixel() {
                bg(c2)
            } else {
                container::Style {
                    background: Some(linear(135.0, c1, c2)),
                    ..Default::default()
                }
            }
        })];
        if let Some(path) = game.and_then(|g| g.covers.hero.as_ref()) {
            layers = layers.push(
                image(image::Handle::from_path(path))
                    .content_fit(ContentFit::Cover)
                    .width(Fill)
                    .height(Fill),
            );
        }
        // Voiles sombres de la maquette : bas de page et côté gauche.
        if !ui.mode.is_pixel() {
            layers =
                layers
                    .push(
                        container(space())
                            .width(Fill)
                            .height(Fill)
                            .style(|_| container::Style {
                                background: Some(linear(
                                    0.0,
                                    theme::BG,
                                    Color {
                                        a: 0.0,
                                        ..theme::BG
                                    },
                                )),
                                ..Default::default()
                            }),
                    )
                    .push(container(space()).width(Fill).height(Fill).style(|_| {
                        container::Style {
                            background: Some(linear(
                                90.0,
                                Color {
                                    a: 0.85,
                                    ..theme::BG
                                },
                                Color {
                                    a: 0.1,
                                    ..theme::BG
                                },
                            )),
                            ..Default::default()
                        }
                    }));
        }
        container(layers.push(body)).width(Fill).height(380).into()
    }

    fn spike_panel(&self, ui: Ui) -> Element<'_, Msg> {
        let mut dots = row![].spacing(6).align_y(iced::Center);
        for a in Accent::ALL {
            let selected = a == self.accent;
            dots = dots.push(
                button(space().width(14).height(14))
                    .on_press(Msg::SetAccent(a))
                    .padding(0)
                    .style(move |_, _| button::Style {
                        background: Some(a.color().into()),
                        border: Border {
                            radius: if ui.mode == Mode::PixelCarre {
                                0.0.into()
                            } else {
                                7.0.into()
                            },
                            width: if selected { 2.0 } else { 0.0 },
                            color: theme::TEXT,
                        },
                        ..Default::default()
                    }),
            );
        }
        let small = |label: String, msg: Msg| {
            button(ui.text(label, 12, Weight::Semibold, theme::SOFT))
                .on_press(msg)
                .padding([6, 10])
                .style(move |_, _| button::Style {
                    background: Some(
                        Color {
                            a: 0.55,
                            ..theme::BG
                        }
                        .into(),
                    ),
                    text_color: theme::SOFT,
                    border: Border {
                        radius: ui.mode.radius(9.0).into(),
                        width: 1.0,
                        color: theme::LINE2,
                    },
                    ..Default::default()
                })
        };
        let panel_row = row![
            ui.mono(t::SPIKE, 10, theme::MUTED),
            dots,
            small(self.mode.name().to_string(), Msg::NextMode),
            small(self.transition_kind.name().to_string(), Msg::NextTransition),
        ]
        .spacing(10)
        .align_y(iced::Center);
        let tip = iced::widget::tooltip(
            container(panel_row).padding([6, 12]).style(move |_| {
                panel(
                    Color {
                        a: 0.55,
                        ..theme::BG
                    },
                    ui.mode.radius(12.0),
                    None,
                )
            }),
            container(ui.text(t::HELP, 12, Weight::Normal, theme::TEXT))
                .padding(8)
                .style(move |_| panel(theme::CARD2, 8.0, None)),
            iced::widget::tooltip::Position::Bottom,
        );
        tip.into()
    }

    fn recent(&self, ui: Ui) -> Element<'_, Msg> {
        let mut picks: Vec<usize> = (0..self.games.len())
            .filter(|&i| self.games[i].last_played.is_some())
            .take(5)
            .collect();
        if picks.is_empty() {
            picks = (0..self.games.len().min(5)).collect();
        }
        let mut cards = row![].spacing(16);
        for i in picks {
            let g = &self.games[i];
            let card = column![
                container(ui.cover(g, f32::INFINITY, 126.0, 12.0, true)).width(Fill),
                row![
                    ui.text(g.store.name(), 12, Weight::Normal, theme::MUTED),
                    space().width(Fill),
                    ui.text(
                        short_playtime(g.playtime_minutes),
                        12,
                        Weight::Normal,
                        theme::MUTED
                    )
                ],
            ]
            .spacing(6)
            .width(Length::FillPortion(1));
            cards = cards.push(mouse_area(card).on_press(Msg::Select(i)));
        }
        column![ui.text(t::RECENT, 17, Weight::Bold, theme::TEXT), cards]
            .spacing(12)
            .padding(Padding::new(40.0).top(22.0))
            .into()
    }
}

fn scan_task(demo: Option<usize>) -> Task<Msg> {
    Task::perform(
        async move {
            let t0 = Instant::now();
            let res = match demo {
                Some(n) => ScanResult {
                    games: fake_games(n),
                    ..Default::default()
                },
                None => launcher_stores::scan(&launcher_stores::detect_roots()),
            };
            (res, t0.elapsed())
        },
        |(res, took)| Msg::Scanned(res, took),
    )
}

/// Petit contexte de style partagé par les fonctions de vue.
#[derive(Clone, Copy)]
struct Ui {
    accent: Color,
    mode: Mode,
}

impl Ui {
    fn text<'a>(
        self,
        s: impl text::IntoFragment<'a>,
        size: u16,
        weight: Weight,
        color: Color,
    ) -> text::Text<'a> {
        text(s)
            .size(f32::from(size))
            .font(self.mode.font(weight))
            .color(color)
    }
    fn mono<'a>(self, s: impl text::IntoFragment<'a>, size: u16, color: Color) -> text::Text<'a> {
        text(s)
            .size(f32::from(size))
            .font(self.mode.mono())
            .color(color)
    }
    fn pill<'a>(self, label: &'a str, on: bool, msg: Msg) -> Element<'a, Msg> {
        button(self.text(
            label,
            12,
            Weight::Semibold,
            if on { theme::BG } else { theme::SOFT },
        ))
        .on_press(msg)
        .padding([7, 12])
        .style(move |_, _| button::Style {
            background: on.then_some(theme::TEXT.into()),
            text_color: if on { theme::BG } else { theme::SOFT },
            border: Border {
                radius: if self.mode == Mode::Normal {
                    15.0.into()
                } else {
                    self.mode.radius(8.0).into()
                },
                width: if on { 0.0 } else { 1.0 },
                color: theme::LINE2,
            },
            ..Default::default()
        })
        .into()
    }
    fn nav<'a>(self) -> Element<'a, Msg> {
        let mut nav = row![].spacing(4);
        for (i, label) in t::NAV.iter().enumerate() {
            let active = i == 0;
            nav = nav.push(
                container(self.text(
                    *label,
                    13,
                    if active {
                        Weight::Bold
                    } else {
                        Weight::Semibold
                    },
                    if active {
                        theme::BG
                    } else {
                        Color {
                            a: 0.6,
                            ..theme::TEXT
                        }
                    },
                ))
                .padding([9, 16])
                .style(move |_| {
                    if active {
                        panel(theme::TEXT, self.mode.radius(9.0), None)
                    } else {
                        container::Style::default()
                    }
                }),
            );
        }
        container(nav)
            .padding(4)
            .style(move |_| {
                panel(
                    Color {
                        a: 0.55,
                        ..theme::BG
                    },
                    self.mode.radius(12.0),
                    None,
                )
            })
            .into()
    }
    fn chip<'a>(self, label: String) -> Element<'a, Msg> {
        let dot = container(space().width(8).height(8)).style(move |_| {
            panel(
                self.accent,
                if self.mode == Mode::PixelCarre {
                    0.0
                } else {
                    4.0
                },
                None,
            )
        });
        container(
            row![
                dot,
                self.text(
                    label,
                    13,
                    Weight::Normal,
                    Color::from_rgb8(0xE8, 0xEA, 0xF2)
                )
            ]
            .spacing(8)
            .align_y(iced::Center),
        )
        .padding([12, 14])
        .style(move |_| {
            panel(
                Color {
                    a: 0.55,
                    ..theme::BG
                },
                self.mode.radius(12.0),
                None,
            )
        })
        .into()
    }
    fn stat<'a>(self, label: &'a str, value: String) -> Element<'a, Msg> {
        container(
            column![
                self.text(
                    label,
                    11,
                    Weight::Normal,
                    Color::from_rgb8(0xC9, 0xCE, 0xDB)
                ),
                self.mono(value, 16, theme::TEXT)
            ]
            .spacing(4),
        )
        .padding([10, 16])
        .width(Length::Shrink)
        .style(move |_| {
            panel(
                Color {
                    a: 0.6,
                    ..theme::BG
                },
                self.mode.radius(12.0),
                None,
            )
        })
        .into()
    }
    /// Jaquette : image Steam si elle existe, sinon dégradé (couleur unie en mode pixel).
    fn cover<'a>(
        self,
        g: &'a InstalledGame,
        w: f32,
        h: f32,
        radius: f32,
        with_title: bool,
    ) -> Element<'a, Msg> {
        let (c1, c2) = theme::cover_colors(&g.title);
        let pixel = self.mode.is_pixel();
        let r = self.mode.radius(radius);
        let width = if w.is_finite() {
            Length::Fixed(w)
        } else {
            Fill
        };
        let base = container(if with_title {
            Element::from(
                container(self.text(g.title.as_str(), 15, Weight::Bold, theme::TEXT))
                    .padding([10, 12])
                    .align_bottom(Fill),
            )
        } else {
            space().into()
        })
        .width(width)
        .height(h)
        .style(move |_| container::Style {
            background: Some(if pixel {
                Background::Color(c1)
            } else {
                linear(150.0, c1, c2)
            }),
            border: Border {
                radius: r.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let path = if with_title {
            g.covers.hero.as_ref()
        } else {
            g.covers.portrait.as_ref()
        };
        match path {
            Some(p) if !pixel => stack![
                base,
                container(
                    image(image::Handle::from_path(p))
                        .content_fit(ContentFit::Cover)
                        .border_radius(Radius::from(r))
                )
                .width(width)
                .height(h)
            ]
            .into(),
            _ => base.into(),
        }
    }
}

fn bg(color: Color) -> container::Style {
    container::Style {
        background: Some(color.into()),
        ..Default::default()
    }
}

fn panel(color: Color, radius: f32, border: Option<Color>) -> container::Style {
    container::Style {
        background: Some(color.into()),
        border: Border {
            radius: radius.into(),
            width: if border.is_some() { 1.0 } else { 0.0 },
            color: border.unwrap_or(Color::TRANSPARENT),
        },
        ..Default::default()
    }
}

fn linear(deg: f32, from: Color, to: Color) -> Background {
    Background::Gradient(Gradient::Linear(
        Linear::new(Degrees(deg))
            .add_stop(0.0, from)
            .add_stop(1.0, to),
    ))
}

fn lighten(c: Color, by: f32) -> Color {
    Color {
        r: (c.r + by).min(1.0),
        g: (c.g + by).min(1.0),
        b: (c.b + by).min(1.0),
        a: c.a,
    }
}

fn playtime(min: Option<u64>) -> String {
    match min {
        Some(m) if m >= 60 => format!("{} h {:02}", m / 60, m % 60),
        Some(m) if m > 0 => format!("{m} min"),
        _ => "—".into(),
    }
}

fn short_playtime(min: Option<u64>) -> String {
    match min {
        Some(m) if m > 0 => format!("{} h", (m + 30) / 60),
        _ => "Installé".into(),
    }
}

fn size(bytes: Option<u64>) -> String {
    match bytes {
        Some(b) if b >= 1_000_000_000 => format!("{:.1} Go", b as f64 / 1e9).replace('.', ","),
        Some(b) if b > 0 => format!("{} Mo", b / 1_000_000),
        _ => "—".into(),
    }
}
