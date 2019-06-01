use std::{
    thread,
    sync::mpsc,
};

enum Event {
    XCBEvent,
}

fn main() {
    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    let window = conn.generate_id();

    let values = [
        (xcb::CW_BACK_PIXMAP, xcb::BACK_PIXMAP_NONE),
        (
            xcb::CW_EVENT_MASK,
            xcb::EVENT_MASK_EXPOSURE
                | xcb::EVENT_MASK_KEY_PRESS
                | xcb::EVENT_MASK_BUTTON_PRESS
                | xcb::EVENT_MASK_BUTTON_RELEASE,
        ),
        (xcb::CW_OVERRIDE_REDIRECT, 1),
    ];

    xcb::create_window(
        &conn,
        xcb::COPY_FROM_PARENT as u8,
        window,
        screen.root(),
        560, 680,
        800, 400,
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(),
        &values,
    );

    let opacity_atom = xcb::intern_atom(&conn, false, "_NET_WM_WINDOW_OPACITY")
        .get_reply()
        .expect("Couldn't create atom _NET_WM_WINDOW_OPACITY")
        .atom();

    let opacity = (0xAAAAAAAAu64 as f64) as u64;

    xcb::change_property(
        &conn,
        xcb::PROP_MODE_REPLACE as u8,
        window,
        opacity_atom,
        xcb::ATOM_CARDINAL,
        32,
        &[opacity],
    );

    xcb::change_property(
        &conn,
        xcb::PROP_MODE_REPLACE as u8,
        window,
        opacity_atom,
        xcb::ATOM_CARDINAL,
        32,
        &[opacity],
    );

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

    let mut visual = find_visual(&conn, screen.root_visual()).unwrap();
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
        800.into(),
        400.into(),
    );

    let cairo_context = cairo::Context::new(&surface);
    conn.flush();

    loop {
        if let Some(event) = conn.poll_for_event() {
            match event.response_type() {
                xcb::EXPOSE => {
                    cairo_context.set_source_rgb(1., 0., 0.);
                    cairo_context.paint();
                },
                _ => {},
            }
        }

        thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn find_visual(conn: &xcb::Connection, visual: xcb::ffi::xcb_visualid_t) -> Option<xcb::Visualtype> {
    for screen in conn.get_setup().roots() {
        for depth in screen.allowed_depths() {
            for vis in depth.visuals() {
                if visual == vis.visual_id() {
                    return Some(vis);
                }
            }
        }
    }
    None
}
