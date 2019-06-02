use std::{
    thread,
    f64::consts::PI,
    time::{Duration, Instant},
};
use chrono::Local;

const FRAME_MS: f32 = 1000. / 60.;

const S_WIDTH: f64 = 1920.;
const S_HEIGHT: f64 = 1200.;

const WIDTH: f64 = S_WIDTH * 0.35;
const HEIGHT: f64 = S_HEIGHT * 0.2;

const FONT: &'static str = "Roboto";
const TIME_FORMAT: &'static str = "%-k:%M";
const DATE_FORMAT: &'static str = "%a, %h %-e %Y";

fn main() {
    let ftime = Duration::from_millis(FRAME_MS as u64);

    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    let window = conn.generate_id();
    let mut visual = find_rgba_visual(&screen).unwrap();

    let colormap = conn.generate_id();
    let cookie = xcb::create_colormap_checked(
        &conn,
        0,
        colormap,
        screen.root(),
        visual.visual_id(),
    );

    if let Err(err) = cookie.request_check() {
        println!("err: {:?}", err.error_code());
    }

    let values = [
        (xcb::CW_BACK_PIXEL, 0x00000000),
        (xcb::CW_BORDER_PIXEL, 0x00000000),
        (xcb::CW_COLORMAP, colormap),
        (
            xcb::CW_EVENT_MASK,
            xcb::EVENT_MASK_EXPOSURE
                | xcb::EVENT_MASK_POINTER_MOTION
                | xcb::EVENT_MASK_BUTTON_1_MOTION
                | xcb::EVENT_MASK_KEY_PRESS
                | xcb::EVENT_MASK_KEY_RELEASE
                | xcb::EVENT_MASK_BUTTON_PRESS
                | xcb::EVENT_MASK_BUTTON_RELEASE,
        ),
        (xcb::CW_OVERRIDE_REDIRECT, 1),
    ];

    let x = S_WIDTH / 2. - WIDTH / 2.;
    let y = S_HEIGHT - HEIGHT - 20.;
    let cookie = xcb::create_window_checked(
        &conn,
        32,
        window,
        screen.root(),
        x as i16, y as i16,
        WIDTH as u16, HEIGHT as u16,
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        visual.visual_id(),
        &values,
    );

    if let Err(err) = cookie.request_check() {
        println!("err: {:?}", err.error_code());
    }

    let type_atom = xcb::intern_atom(&conn, false, "_NET_WM_WINDOW_TYPE")
        .get_reply()
        .expect("Couldn't create atom _NET_WM_WINDOW_TYPE")
        .atom();

    let dock_atom = xcb::intern_atom(&conn, false, "_NET_WM_WINDOW_TYPE_DOCK")
        .get_reply()
        .expect("Couldn't create atom _NET_WM_WINDOW_DOCK")
        .atom();

    xcb::change_property(
        &conn,
        xcb::PROP_MODE_REPLACE as u8,
        window,
        type_atom,
        xcb::ATOM_ATOM,
        8,
        &[dock_atom],
    );

    xcb::map_window(&conn, window);
    conn.flush();

    let cairo_xcb_conn = unsafe {
        cairo::XCBConnection::from_raw_none(
            conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t
        )
    };
    let cairo_xcb_drawable = cairo::XCBDrawable(window);
    let raw_visualtype = &mut visual.base as *mut xcb::ffi::xcb_visualtype_t;
    let cairo_xcb_visual = unsafe {
        cairo::XCBVisualType::from_raw_none(raw_visualtype as *mut cairo_sys::xcb_visualtype_t)
    };
    let surface = <cairo::Surface as cairo::XCBSurface>::create(
        &cairo_xcb_conn,
        &cairo_xcb_drawable,
        &cairo_xcb_visual,
        WIDTH as i32,
        HEIGHT as i32,
    );

    let cairo_context = cairo::Context::new(&surface);

    let current_time = Local::now();
    let mut state = State {
        time: current_time.format(TIME_FORMAT).to_string(),
        date: current_time.format(DATE_FORMAT).to_string(),

        volume: 50,
        volume_hovered: false,
        volume_pressed: false,

        brightness: 50,
        brightness_hovered: false,
        brightness_pressed: false,
    };

    let volume_bounds = (
        (WIDTH * (1. / 3.) + 14.), (HEIGHT * 0.75 + 14.),
        (WIDTH * (2. / 3.) - 28.), (HEIGHT * 0.25 - 28.),
    );

    let brightness_bounds = (
        (WIDTH * (1. / 3.) + 14.), (HEIGHT * 0.5 + 14.),
        (WIDTH * (2. / 3.) - 28.), (HEIGHT * 0.25 - 28.),
    );

    let mut clock_timer = Instant::now();
    let mut dragging = false;
    loop {
        if clock_timer.elapsed().as_millis() > 1_000 {
            clock_timer = Instant::now();
            let current_time = Local::now();
            state.time = current_time.format(TIME_FORMAT).to_string();
            state.date = current_time.format(DATE_FORMAT).to_string();

            draw(&cairo_context, &state);
            conn.flush();
        }

        let fstart = Instant::now();
        while let Some(event) = conn.poll_for_event() {
            match event.response_type() {
                xcb::EXPOSE => {
                    draw(&cairo_context, &state);
                    conn.flush();
                },
                xcb::KEY_PRESS => {
                    let ev: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&event) };
                    let syms = xcb_util::keysyms::KeySymbols::new(&conn);
                    let ksym = syms.press_lookup_keysym(ev, 0);
                    println!("pressed {:?}", ksym);
                },
                xcb::BUTTON_PRESS => {
                    let ev: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&event) };
                    if ev.detail() == 1 {
                        let (mx, my) = (ev.root_x() - (x as i16), ev.root_y() - (y as i16));
                        let pos = (mx as f64, my as f64);

                        if contains(pos, volume_bounds) {
                            dragging = true;
                            state.volume_pressed = true;
                            let current = pos.0 - volume_bounds.0;
                            state.volume = ((current / volume_bounds.2) * 100.) as u32;

                            draw(&cairo_context, &state);
                            conn.flush();
                        } else if contains(pos, brightness_bounds) {
                            dragging = true;
                            state.brightness_pressed = true;
                            let current = pos.0 - volume_bounds.0;
                            state.brightness = ((current / brightness_bounds.2) * 100.) as u32;

                            draw(&cairo_context, &state);
                            conn.flush();
                        }
                    }
                },
                xcb::BUTTON_RELEASE => {
                    let ev: &xcb::ButtonReleaseEvent = unsafe { xcb::cast_event(&event) };
                    let detail = unsafe { (*ev.ptr).detail };
                    if detail == 1 {
                        dragging = false;
                        state.volume_pressed = false;
                        state.brightness_pressed = false;
                    }
                }
                xcb::MOTION_NOTIFY => {
                    let ev: &xcb::MotionNotifyEvent = unsafe { xcb::cast_event(&event) };
                    let (mx, my) = (ev.root_x() - (x as i16), ev.root_y() - (y as i16));
                    let pos = (mx as f64, my as f64);

                    state.volume_hovered = false;
                    state.brightness_hovered = false;

                    if !dragging {
                        if contains(pos, volume_bounds) {
                            state.volume_hovered = true;
                        } else if contains(pos, brightness_bounds) {
                            state.brightness_hovered = true;
                        }

                        draw(&cairo_context, &state);
                        conn.flush();
                    } else if state.volume_pressed {
                        let current = pos.0 - volume_bounds.0;
                        state.volume = ((current / volume_bounds.2) * 100.) as u32;

                        draw(&cairo_context, &state);
                        conn.flush();
                    } else if state.brightness_pressed {
                        let current = pos.0 - volume_bounds.0;
                        state.brightness = ((current / brightness_bounds.2) * 100.) as u32;

                        draw(&cairo_context, &state);
                        conn.flush();
                    }
                },
                _ => {},
            }
        }

        let diff = ftime - fstart.elapsed();
        if diff.as_millis() > 0 {
            thread::sleep(diff);
        }
    }
}

fn contains(pos: (f64, f64), rect: (f64, f64, f64, f64)) -> bool {
    pos.0 >= rect.0 && pos.0 <= rect.0 + rect.2 &&
        pos.1 >= rect.1 && pos.1 <= rect.1 + rect.3 
}

struct State {
    pub time: String,
    pub date: String,

    pub volume: u32,
    pub volume_hovered: bool,
    pub volume_pressed: bool,

    pub brightness: u32,
    pub brightness_hovered: bool,
    pub brightness_pressed: bool,
}

fn draw(ctx: &cairo::Context, state: &State) {
    ctx.push_group();

    let panel_radius = 6.;
    let panel_padding = 4.;
    let panel_color = (0.3, 0.3, 0.3, 1.0);

    let vol_perc = state.volume as f64 / 100.;
    let bright_perc = state.brightness as f64 / 100.;

    let vol_color = if state.volume_hovered {
        (1.0, 0.3, 0.3, 1.0)
    } else if state.volume_pressed {
        (0.9, 0.2, 0.2, 1.0)
    } else {
        (1.0, 0.3, 0.3, 0.6)
    };

    let bright_color = if state.brightness_hovered {
        (1.0, 1., 0.3, 1.0)
    } else if state.brightness_pressed {
        (0.9, 0.9, 0.2, 1.0)
    } else {
        (1.0, 1., 0.3, 0.6)
    };

    // clock
    draw_rounded_rect(
        &ctx,
        0., HEIGHT * 0.5,
        WIDTH * (1. / 3.), HEIGHT * 0.5,
        panel_radius, panel_padding,
        panel_color,
    );

    // volume
    draw_rounded_rect(
        &ctx,
        WIDTH * (1. / 3.), HEIGHT * 0.75,
        WIDTH * (2. / 3.), HEIGHT * 0.25,
        panel_radius, panel_padding,
        panel_color,
    );

    draw_rounded_rect(
        &ctx,
        WIDTH * (1. / 3.), HEIGHT * 0.75,
        WIDTH * (2. / 3.) * vol_perc, HEIGHT * 0.25,
        panel_radius, 14.,
        vol_color,
    );

    // brightness
    draw_rounded_rect(
        &ctx,
        WIDTH * (1. / 3.), HEIGHT * 0.5,
        WIDTH * (2. / 3.), HEIGHT * 0.25,
        panel_radius, panel_padding,
        panel_color,
    );

    draw_rounded_rect(
        &ctx,
        WIDTH * (1. / 3.), HEIGHT * 0.5,
        WIDTH * (2. / 3.) * bright_perc, HEIGHT * 0.25,
        panel_radius, 14.,
        bright_color,
    );

    draw_rounded_rect(
        &ctx,
        0., 0.,
        WIDTH * 0.5, HEIGHT * 0.5,
        panel_radius, panel_padding,
        panel_color,
    );

    draw_rounded_rect(
        &ctx,
        WIDTH * 0.5, 0.,
        WIDTH * 0.5, HEIGHT * 0.5,
        panel_radius, panel_padding,
        panel_color,
    );

    draw_text_centered(
        &ctx,
        50.,
        WIDTH * (1. / 3.) * 0.5, HEIGHT * 0.75 - 10.,
        &state.time,
        (1., 1., 1., 1.),
    );

    draw_text_centered(
        &ctx,
        14.,
        WIDTH * (1. / 3.) * 0.5, HEIGHT * 0.75 + 30.,
        &state.date,
        (1., 1., 1., 1.),
    );

    ctx.pop_group_to_source();
    ctx.paint();
}

fn find_rgba_visual(screen: &xcb::Screen) -> Option<xcb::Visualtype> {
    screen.allowed_depths()
        .filter(|it| it.depth() == 32)
        .flat_map(|it| it.visuals())
        .filter(|it| it.class() == 4)
        .nth(0)
}

fn draw_text_centered(ctx: &cairo::Context, s: f64, x: f64, y: f64, text: &str, c: (f64, f64, f64, f64)) {
    ctx.select_font_face(FONT, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    ctx.set_font_size(s);

    let extents = ctx.text_extents(text);
    let x = x - extents.width / 2.;
    let y = y + extents.height / 2.;

    ctx.move_to(x, y);
    ctx.set_source_rgba(c.0, c.1, c.2, c.3);
    ctx.show_text(text);
}

fn draw_rounded_rect(ctx: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64, p: f64, c: (f64, f64, f64, f64)) {
    let deg = PI / 180.;

    let x = x + p;
    let y = y + p;
    let mut w = w - 2. * p;
    let h = h - 2. * p;

    if w < r {
        w = r;
    }

    ctx.new_sub_path();
    ctx.arc(x + w - r, y + r,     r, -90. * deg, 0. * deg);
    ctx.arc(x + w - r, y + h - r, r, 0. * deg,   90. * deg);
    ctx.arc(x + r,     y + h - r, r, 90. * deg,  180. * deg);
    ctx.arc(x + r,     y + r,     r, 180. * deg, 270. * deg);
    ctx.close_path();

    ctx.set_source_rgba(c.0, c.1, c.2, c.3);
    ctx.fill();
}

