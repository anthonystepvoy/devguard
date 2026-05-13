pub fn init(no_color: bool) {
    if no_color {
        colored::control::set_override(false);
    } else {
        #[cfg(windows)]
        {
            let _ = colored::control::set_virtual_terminal(true);
        }
    }
}
