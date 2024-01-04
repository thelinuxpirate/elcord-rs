use emacs::{
    defun,
    Env,
    Result,
    Value
};

//use x11rb::protocol::xproto::get_input_focus;
//use x11rb::protocol::xproto;
use std::str;
use xcb::x;

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

// grabs title of current X window & prints it
#[defun]
fn print_xorg_window(env: &Env) -> Result<Value<'_>> {
    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();

    let wm_client_list = conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: "_NET_CLIENT_LIST".as_bytes(),
    });
    let wm_client_list = conn.wait_for_reply(wm_client_list)?.atom();
    assert!(wm_client_list != x::ATOM_NONE, "EWMH not supported");

    let mut titles = Vec::new();

    for screen in setup.roots() {
        let window = screen.root();

        let pointer = conn.send_request(&x::QueryPointer { window });
        let pointer = conn.wait_for_reply(pointer)?;

        if pointer.same_screen() {
            let list = conn.send_request(&x::GetProperty {
                delete: false,
                window,
                property: wm_client_list,
                r#type: x::ATOM_NONE,
                long_offset: 0,
                long_length: 100,
            });
            let list = conn.wait_for_reply(list)?;

            for client in list.value::<x::Window>() {
                let cookie = conn.send_request(&x::GetProperty {
                    delete: false,
                    window: *client,
                    property: x::ATOM_WM_NAME,
                    r#type: x::ATOM_STRING,
                    long_offset: 0,
                    long_length: 1024,
                });
                let reply = conn.wait_for_reply(cookie)?;
                let title = reply.value();
                let title = str::from_utf8(title).expect("invalid UTF-8");
                let title_display = format!("{}", title);
                titles.push(title_display);
            }
        }
    }

    let current_win = titles.join("");
    println!("{current_win}");
    env.message(&format!("{}", current_win))
}
