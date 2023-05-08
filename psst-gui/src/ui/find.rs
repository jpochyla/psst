use druid::{
    widget::{prelude::*, Controller, Either, Flex, Label, TextBox},
    KbKey, Selector, WidgetExt,
};

use crate::{
    cmd,
    controller::InputController,
    data::{FindQuery, Finder, MatchFindQuery},
    ui::theme,
    widget::{Empty, MyWidgetExt},
};

#[derive(Clone)]
pub struct Find {
    sender: WidgetId,
    query: FindQuery,
}

#[derive(Clone)]
struct Report {
    sender: WidgetId,
}

const FIND: Selector = Selector::new("find");
const REPORT_MATCH: Selector<Report> = Selector::new("report-match");
const FOCUS_MATCH: Selector = Selector::new("focus-match");

pub struct Findable<W> {
    inner: W,
    selector: Selector<Find>,
    is_matching: bool,
}

impl<W> Findable<W> {
    pub fn new(inner: W, selector: Selector<Find>) -> Self {
        Self {
            inner,
            selector,
            is_matching: false,
        }
    }

    fn set_state(&mut self, ctx: &mut EventCtx, matches: bool) {
        if self.is_matching != matches {
            self.is_matching = matches;
            ctx.request_paint();
        }
    }
}

impl<T, W> Widget<T> for Findable<W>
where
    W: Widget<T>,
    T: MatchFindQuery,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(self.selector) => {
                let Find { sender, query } = cmd.get_unchecked(self.selector);
                self.set_state(
                    ctx,
                    if query.is_empty() {
                        false
                    } else {
                        data.matches_query(query)
                    },
                );
                if self.is_matching {
                    let report = Report {
                        sender: ctx.widget_id(),
                    };
                    ctx.submit_command(REPORT_MATCH.with(report).to(*sender));
                }
            }
            Event::Command(cmd) if cmd.is(FOCUS_MATCH) => {
                ctx.scroll_to_view();
            }
            _ => {}
        }
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.is_matching {
            let bounds = ctx
                .size()
                .to_rect()
                .inset(-2.0)
                .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));
            ctx.fill(bounds, &env.get(theme::GREY_500));
        }
        self.inner.paint(ctx, data, env);
    }
}

pub fn finder_widget(selector: Selector<Find>, label: &'static str) -> impl Widget<Finder> {
    let input_id = WidgetId::next();

    let input = TextBox::new()
        .with_placeholder(label)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .controller(InputController::new())
        .with_id(input_id)
        .expand_width()
        .lens(Finder::query);

    let not_found = Label::new("Not Found")
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR);

    let results = Label::dynamic(|finder: &Finder, _| {
        format!("{} / {}", finder.focused_result + 1, finder.results)
    })
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .with_text_color(theme::PLACEHOLDER_COLOR);

    let previous = Label::new("‹")
        .padding(theme::grid(0.5))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|_, _, data: &mut Finder, _| data.focus_previous());

    let next = Label::new("›")
        .padding(theme::grid(0.5))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|_, _, data: &mut Finder, _| data.focus_next());

    let results_with_controls = Either::new(
        |data, _| data.results > 0,
        Flex::row()
            .with_child(previous)
            .with_spacer(theme::grid(0.5))
            .with_child(results)
            .with_spacer(theme::grid(0.5))
            .with_child(next),
        not_found,
    );

    let finder = Flex::row()
        .with_flex_child(input, 1.0)
        .with_default_spacer()
        .with_child(results_with_controls)
        .padding(theme::grid(1.0))
        .background(theme::GREY_600);

    Either::new(|data, _| data.show, finder, Empty)
        .controller(FinderController { selector, input_id })
}

struct FinderController {
    selector: Selector<Find>,
    input_id: WidgetId,
}

impl<W> Controller<Finder, W> for FinderController
where
    W: Widget<Finder>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Finder,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(FIND) => {
                data.reset_matches();
                ctx.submit_command(self.selector.with(Find {
                    sender: ctx.widget_id(),
                    query: FindQuery::new(&data.query),
                }));
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(REPORT_MATCH) => {
                if data.report_match() == data.focused_result {
                    ctx.submit_command(FOCUS_MATCH.to(cmd.get_unchecked(REPORT_MATCH).sender));
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::TOGGLE_FINDER) => {
                data.reset();
                data.show = !data.show;
                if data.show {
                    ctx.submit_command(cmd::SET_FOCUS.to(self.input_id));
                }
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if k_e.key == KbKey::Escape => {
                data.show = false;
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &Finder,
        data: &Finder,
        env: &Env,
    ) {
        if !old_data.query.same(&data.query) || !old_data.focused_result.same(&data.focused_result)
        {
            ctx.submit_command(FIND.to(ctx.widget_id()));
        }
        child.update(ctx, old_data, data, env)
    }
}
