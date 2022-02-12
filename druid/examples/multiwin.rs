// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Opening and closing windows and using window and context menus.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::prelude::*;
use druid::widget::{
    Align, BackgroundBrush, Button, Controller, ControllerHost, Flex, Label, Padding,
};
use druid::Target::Global;
use druid::{
    commands as sys_cmds, AppDelegate, AppLauncher, Application, Color, Command, Data, DelegateCtx,
    Handled, LocalizedString, Menu, MenuItem, Target, WindowDesc, WindowId,
};
use tracing::info;

#[derive(Debug, Clone, Default, Data)]
struct State {
    menu_count: usize,
    selected: usize,
    glow_hot: bool,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder()).menu(make_menu).title(
        LocalizedString::new("multiwin-demo-window-title").with_placeholder("Many windows!"),
    );
    AppLauncher::with_window(main_window)
        .delegate(Delegate {
            windows: Vec::new(),
        })
        .log_to_console()
        .launch(State::default())
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<State> {
    let text = LocalizedString::new("hello-counter")
        .with_arg("count", |data: &State, _env| data.menu_count.into());
    let label = Label::new(text);
    let inc_button =
        Button::<State>::new("Add menu item").on_click(|_ctx, data, _env| data.menu_count += 1);
    let dec_button = Button::<State>::new("Remove menu item")
        .on_click(|_ctx, data, _env| data.menu_count = data.menu_count.saturating_sub(1));
    let new_button = Button::<State>::new("New window").on_click(|ctx, _data, _env| {
        ctx.submit_command(sys_cmds::NEW_FILE.to(Global));
    });
    let quit_button = Button::<State>::new("Quit app").on_click(|_ctx, _data, _env| {
        Application::global().quit();
    });

    let mut col = Flex::column();
    col.add_flex_child(Align::centered(Padding::new(5.0, label)), 1.0);
    let mut row = Flex::row();
    row.add_child(Padding::new(5.0, inc_button));
    row.add_child(Padding::new(5.0, dec_button));
    col.add_flex_child(Align::centered(row), 1.0);
    let mut row = Flex::row();
    row.add_child(Padding::new(5.0, new_button));
    row.add_child(Padding::new(5.0, quit_button));
    col.add_flex_child(Align::centered(row), 1.0);
    let content = ControllerHost::new(col, ContextMenuController);
    Glow::new(content)
}

struct Glow<W> {
    inner: W,
}

impl<W> Glow<W> {
    pub fn new(inner: W) -> Glow<W> {
        Glow { inner }
    }
}

impl<W: Widget<State>> Widget<State> for Glow<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut State, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &State, env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &State, data: &State, env: &Env) {
        if old_data.glow_hot != data.glow_hot {
            ctx.request_paint();
        }
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &State,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &State, env: &Env) {
        if data.glow_hot && ctx.is_hot() {
            BackgroundBrush::Color(Color::rgb8(200, 55, 55)).paint(ctx, data, env);
        }
        self.inner.paint(ctx, data, env);
    }
}

struct ContextMenuController;
struct Delegate {
    windows: Vec<WindowId>,
}

impl<W: Widget<State>> Controller<State, W> for ContextMenuController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut State,
        env: &Env,
    ) {
        match event {
            Event::MouseDown(ref mouse) if mouse.button.is_right() => {
                ctx.show_context_menu(make_context_menu(), mouse.pos);
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

impl AppDelegate<State> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> Handled {
        if cmd.is(sys_cmds::NEW_FILE) {
            #[allow(unused)]
            let mut new_win = WindowDesc::new(ui_builder())
                .menu(make_menu)
                .window_size((data.selected as f64 * 100.0 + 300.0, 500.0));
            #[cfg(target_family = "wasm")]
            {
                // this is a web application, so use window.open() to open a new window and create
                // a new canvas there for the new druid window
                use wasm_bindgen::JsCast;
                let this_window = web_sys::window().expect("Window variable unset");
                let new_window = this_window
                    .open_with_url_and_target_and_features("", "", "popup")
                    .expect("window.open() threw error");
                let new_window = match new_window {
                    Some(new_window) => new_window,
                    None => {
                        info!("Couldn't open new window -- popup blocker?");
                        return Handled::Yes;
                    }
                };

                let new_document = new_window.document().expect("new_window has no document");
                let canvas_element = new_document
                    .create_element("canvas")
                    .expect("Couldn't create canvas")
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .expect("Couldn't cast canvas HtmlElement to HtmlCanvasElement");
                let body_element = new_document.body().expect("new_document has no body");
                body_element
                    .append_with_node_1(&canvas_element)
                    .expect("Couldn't append canvas");

                // set CSS so that the canvas always fills the entire window
                fn make_element_fullscreen(element: &web_sys::HtmlElement) {
                    for (property, value) in [
                        ("margin", "0px"),
                        ("padding", "0px"),
                        ("width", "100%"),
                        ("height", "100%"),
                        ("overflow", "hidden"),
                    ] {
                        element
                            .style()
                            .set_property(property, value)
                            .expect(&format!("Setting property {} failed", property))
                    }
                }
                let document_element = new_document
                    .document_element()
                    .expect("new_document has no documentElement")
                    .dyn_into::<web_sys::HtmlElement>()
                    .expect("Couldn't cast document.documentElement to HtmlElement");
                make_element_fullscreen(&document_element);
                make_element_fullscreen(&body_element);
                make_element_fullscreen(&canvas_element);

                new_win = new_win.canvas_element(canvas_element);
            }
            ctx.new_window(new_win);
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn window_added(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        info!("Window added, id: {:?}", id);
        self.windows.push(id);
    }

    fn window_removed(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        info!("Window removed, id: {:?}", id);
        if let Some(pos) = self.windows.iter().position(|x| *x == id) {
            self.windows.remove(pos);
        }
    }
}

#[allow(unused_assignments)]
fn make_menu(_: Option<WindowId>, state: &State, _: &Env) -> Menu<State> {
    let mut base = Menu::empty();
    #[cfg(target_os = "macos")]
    {
        base = druid::platform_menus::mac::menu_bar();
    }
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "openbsd"))]
    {
        base = base.entry(druid::platform_menus::win::file::default());
    }
    if state.menu_count != 0 {
        let mut custom = Menu::new(LocalizedString::new("Custom"));

        for i in 1..=state.menu_count {
            custom = custom.entry(
                MenuItem::new(
                    LocalizedString::new("hello-counter")
                        .with_arg("count", move |_: &State, _| i.into()),
                )
                .on_activate(move |_ctx, data, _env| data.selected = i)
                .enabled_if(move |_data, _env| i % 3 != 0)
                .selected_if(move |data, _env| i == data.selected),
            );
        }
        base = base.entry(custom);
    }
    base.rebuild_on(|old_data, data, _env| old_data.menu_count != data.menu_count)
}

fn make_context_menu() -> Menu<State> {
    Menu::empty()
        .entry(
            MenuItem::new(LocalizedString::new("Increment"))
                .on_activate(|_ctx, data: &mut State, _env| data.menu_count += 1),
        )
        .entry(
            MenuItem::new(LocalizedString::new("Decrement")).on_activate(
                |_ctx, data: &mut State, _env| data.menu_count = data.menu_count.saturating_sub(1),
            ),
        )
        .entry(
            MenuItem::new(LocalizedString::new("Glow when hot"))
                .on_activate(|_ctx, data: &mut State, _env| data.glow_hot = !data.glow_hot),
        )
}
