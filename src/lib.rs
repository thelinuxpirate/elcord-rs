// TODO | After Snormacs-RS come back and finish this
use emacs::{
		defun,
		Env,
		Result,
		Value
};

use xcb::{
		x,
		Xid
};

// Emacs won't load the module without this.
emacs::plugin_is_GPL_compatible!();

// Register the initialization hook that Emacs will call when it loads the module.
#[emacs::module(name = "elcord-rs")]
fn init(env: &Env) -> Result<Value<'_>> {
		env.message("Loaded \"Elcord-RS\"!")
}

// Define a function callable by Lisp code.
#[defun]
fn init_message(env: &Env) -> Result<Value<'_>> {
		let init_msg: &str = "Initialization Status: [O K]";
		env.message(&format!("{}", init_msg))
}

// starts a connection to the X server
#[defun]
fn read_xorg_windows() -> xcb::Result<()> {
	  // Connect to the X server.
    let (conn, screen_num) = xcb::Connection::connect(None)?;

    // Fetch the `x::Setup` and get the main `x::Screen` object.
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    // Generate an `Xid` for the client window.
    // The type inference is needed here.
    let window: x::Window = conn.generate_id();

    // We can now create a window. For this we pass a `Request`
    // object to the `send_request_checked` method. The method
    // returns a cookie that will be used to check for success.
    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: window,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 150,
        height: 150,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        // this list must be in same order than `Cw` enum order
        value_list: &[
            x::Cw::BackPixel(screen.white_pixel()),
            x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS)
        ],
    });
    // We now check if the window creation worked.
    // A cookie can't be cloned; it is moved to the function.
    conn.check_request(cookie)?;

    // Let's change the window title
    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: x::ATOM_WM_NAME,
        r#type: x::ATOM_STRING,
        data: b"My XCB Window",
    });
    // And check for success again
    conn.check_request(cookie)?;

    // We now show ("map" in X terminology) the window.
    // This time we do not check for success, so we discard the cookie.
    conn.send_request(&x::MapWindow {
        window,
    });

    // We need a few atoms for our application.
    // We send a few requests in a row and wait for the replies after.
    let (wm_protocols, wm_del_window, wm_state, wm_state_maxv, wm_state_maxh) = {
        let cookies = (
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"WM_PROTOCOLS",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"WM_DELETE_WINDOW",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MAXIMIZED_VERT",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MAXIMIZED_HORZ",
            }),
        );
        (
            conn.wait_for_reply(cookies.0)?.atom(),
            conn.wait_for_reply(cookies.1)?.atom(),
            conn.wait_for_reply(cookies.2)?.atom(),
            conn.wait_for_reply(cookies.3)?.atom(),
            conn.wait_for_reply(cookies.4)?.atom(),
        )
    };

    // We now activate the window close event by sending the following request.
    // If we don't do this we can still close the window by clicking on the "x" button,
    // but the event loop is notified through a connection shutdown error.
    conn.check_request(conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: wm_protocols,
        r#type: x::ATOM_ATOM,
        data: &[wm_del_window],
    }))?;

    // Previous request was checked, so a flush is not necessary in this case.
    // Otherwise, here is how to perform a connection flush.
    conn.flush()?;

    let mut maximized = false;

    // We enter the main event loop
    loop {
        match conn.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(ev)) => {
                if ev.detail() == 0x3a {
                    // The M key was pressed
                    // (M only on qwerty keyboards. Keymap support is done
                    // with the `xkb` extension and the `xkbcommon-rs` crate)

                    // We toggle maximized state, for this we send a message
                    // by building a `x::ClientMessageEvent` with the proper
                    // atoms and send it to the server.

                    let data = x::ClientMessageData::Data32([
                        if maximized { 0 } else { 1 },
                        wm_state_maxv.resource_id(),
                        wm_state_maxh.resource_id(),
                        0,
                        0,
                    ]);
                    let event = x::ClientMessageEvent::new(window, wm_state, data);
                    let cookie = conn.send_request_checked(&x::SendEvent {
                        propagate: false,
                        destination: x::SendEventDest::Window(screen.root()),
                        event_mask: x::EventMask::STRUCTURE_NOTIFY,
                        event: &event,
                    });
                    conn.check_request(cookie)?;

                    // Same as before, if we don't check for error, we have to flush
                    // the connection.
                    // conn.flush()?;

                    maximized = !maximized;
                } else if ev.detail() == 0x18 {
                    // Q (on qwerty)

                    // We exit the event loop (and the program)
                    break Ok(());
                }
            }
            xcb::Event::X(x::Event::ClientMessage(ev)) => {
                // We have received a message from the server
                if let x::ClientMessageData::Data32([atom, ..]) = ev.data() {
                    if atom == wm_del_window.resource_id() {
                        // The received atom is "WM_DELETE_WINDOW".
                        // We can check here if the user needs to save before
                        // exit, or in our case, exit right away.
                        break Ok(());
                    }
                }
            }
            _ => {}
        }
    }	
}
